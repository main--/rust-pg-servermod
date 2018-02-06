use std::os::raw::c_void;
use std::marker::PhantomData;
use std::{ptr, mem};
use types::Oid;
use error;
use heap::{Heap, HeapTuple, HeapTupleData};
use relation::{Relation, GetTransactionSnapshot};

type IndexScanDesc = *mut c_void;
extern "C" {
    fn index_open(relation: Oid, lockmode: i32 /* set to 1 */) -> *const Relation;
    fn index_close(relation: *const Relation, lockmode: i32);
    fn index_beginscan(heap: *const Relation, index: *const Relation, snapshot: *mut c_void /* null */, nkeys: i32, norderbys: i32) -> IndexScanDesc;
    // afaik orderbys are broken and unused
    fn index_rescan(scan: IndexScanDesc, scankeys: *const ScanKey, nkeys: i32, orderbys: *mut u8, norderbys: i32);
    fn index_getnext(scan: IndexScanDesc, direction: i32) -> *mut HeapTupleData<'static>; // returns HeapTuple
    //fn index_getnext_tid(scan: IndexScanDesc, direction: i32) -> *mut c_void; // returns ItemPointer
    fn index_endscan(scan: IndexScanDesc);

    fn ScanKeyInit(entry: *mut ScanKey, attr_num: u16, strat_num: u16, regproc: u32, arg: usize);
}

#[repr(C)]
pub struct ScanKey {
    _pad: [u8; ::LEN_SCANKEYDATA],
}

impl ScanKey {
    // TODO: value is a Datum
    pub fn new(column: u16, value: usize) -> ScanKey {
        unsafe {
            error::convert_postgres_error(|| {
                let btint4cmp = 184;
                let mut buf: ScanKey = mem::uninitialized();
                ScanKeyInit(&mut buf, column, 3, btint4cmp, value);
                buf
            })
        }
    }
}

// FIXME: this code works with btree index only.
//        need to figure out how this /actually/ works
// FIXME: we only support eq right now, but gt/lt should be easy to do hopefully?
// FIXME: validate index structure (postgres does not guard against invalid scankeys,
//        most importantly column id needs to be valid or things break horribly)

pub struct IndexScan<'a> {
    ptr: IndexScanDesc,
    marker: PhantomData<&'a Index>,
    heap: &'a Heap,
}
impl<'a> Iterator for IndexScan<'a> {
    type Item = HeapTuple<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let tuple = error::convert_postgres_error(|| index_getnext(self.ptr, 1));
            if tuple.is_null() {
                None
            } else {
                Some(HeapTuple {
                    data: *tuple,
                    tupledesc: (*self.heap.0).td,
                })
            }
        }
    }
}
impl<'a> Drop for IndexScan<'a> {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| index_endscan(self.ptr))
        }
    }
}

pub struct Index(*const Relation);
impl Index {
    pub fn open(oid: Oid) -> Index {
        unsafe {
            error::convert_postgres_error(|| Index(index_open(oid, 1)))
        }
    }

    pub fn scan<'a>(&'a self, heap: &'a Heap, scankeys: &'a [ScanKey]) -> IndexScan<'a> {
        assert!(scankeys.len() <= 1);

        unsafe {
            error::convert_postgres_error(|| {
                let snap = GetTransactionSnapshot();
                assert!(!snap.is_null());

                let intlen = scankeys.len() as i32;
                let scan = index_beginscan(heap.0, self.0, snap, intlen, 0);
                index_rescan(scan, scankeys.as_ptr(), intlen, ptr::null_mut(), 0);
                IndexScan {
                    ptr: scan,
                    marker: PhantomData,
                    heap,
                }
            })
        }
    }
}
impl Drop for Index {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| index_close(self.0, 1))
        }
    }
}
