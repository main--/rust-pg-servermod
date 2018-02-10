use std::os::raw::c_void;
use std::marker::PhantomData;
use std::{ptr, mem};

use types::Oid;
use error;
use heap::Heap;
use relation::{Relation, GetTransactionSnapshot};
use alloc::MemoryContext;
use tupledesc::RefTupleDesc;
use tupleslot::{EmptyTupleSlot, TupleSlot};

#[repr(C)]
struct IndexScanDescData {
    _pad: [u8; ::XS_CBUF_OFFSET],
    xs_cbuf: i32,
}
type IndexScanDesc = *mut IndexScanDescData;
extern "C" {
    fn index_open(relation: Oid, lockmode: i32 /* set to 1 */) -> *const Relation;
    fn index_close(relation: *const Relation, lockmode: i32);
    fn index_beginscan(heap: *const Relation, index: *const Relation, snapshot: *mut c_void /* null */, nkeys: i32, norderbys: i32) -> IndexScanDesc;
    // afaik orderbys are broken and unused
    fn index_rescan(scan: IndexScanDesc, scankeys: *const ScanKey, nkeys: i32, orderbys: *mut u8, norderbys: i32);
    fn index_getnext(scan: IndexScanDesc, direction: i32) -> *mut c_void;
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

pub struct RawIndexScan<'a> {
    ptr: IndexScanDesc,
    marker: PhantomData<&'a Index>,
}
impl<'a> Iterator for RawIndexScan<'a> {
    type Item = *const c_void;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let tuple = error::convert_postgres_error(|| index_getnext(self.ptr, 1));
            if tuple.is_null() {
                None
            } else {
                Some(tuple)
            }
        }
    }
}
impl<'a> Drop for RawIndexScan<'a> {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| index_endscan(self.ptr))
        }
    }
}


pub struct IndexScan<'alloc, 'a> {
    raw: RawIndexScan<'a>,
    slot: EmptyTupleSlot<'alloc, RefTupleDesc<'a>>,
}

impl<'alloc, 'a> IndexScan<'alloc, 'a> {
    pub fn next<'b>(&'b mut self) -> Option<TupleSlot<'alloc, 'b, 'a, RefTupleDesc<'a>>> {
        unsafe {
            match self.raw.next() {
                None => None,
                Some(tuple) => {
                    let buffer = (*self.raw.ptr).xs_cbuf;
                    Some(self.slot.store_tuple(tuple, buffer))
                }
            }
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

    pub fn scan<'alloc, 'a>(&'a self,
                            heap: &'a Heap,
                            scankeys: &'a [ScanKey],
                            alloc: &'alloc MemoryContext<'alloc>) -> IndexScan<'alloc, 'a> {
        IndexScan {
            raw: self.scan_raw(heap, scankeys),
            slot: EmptyTupleSlot::create(heap.tuple_desc(), alloc),
        }
    }

    pub fn scan_raw<'a>(&'a self, heap: &'a Heap, scankeys: &'a [ScanKey]) -> RawIndexScan<'a> {
        assert!(scankeys.len() <= 1);

        unsafe {
            error::convert_postgres_error(|| {
                let snap = GetTransactionSnapshot();
                assert!(!snap.is_null());

                let intlen = scankeys.len() as i32;
                let scan = index_beginscan(heap.0, self.0, snap, intlen, 0);
                index_rescan(scan, scankeys.as_ptr(), intlen, ptr::null_mut(), 0);
                RawIndexScan {
                    ptr: scan,
                    marker: PhantomData,
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
