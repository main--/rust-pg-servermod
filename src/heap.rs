use std::os::raw::c_void;
use std::marker::PhantomData;
use std::ptr;
use types::Oid;
use error;
use Datum;

#[repr(C, packed)]
struct Relation {
    _pad: [u8; ::RELATT_OFFSET],
    td: *const RawTupleDesc,
}
type HeapScanDesc = *mut c_void;

#[repr(C, packed)]
struct RawTupleDesc {
    natts: i32,
    tdtypeid: Oid,
    // ...
}
extern "C" {
    fn heap_open(relation: Oid, lockmode: i32) -> *const Relation;
    fn relation_close(relation: *const Relation, lockmode: i32);

    fn heap_beginscan(relation: *const Relation, snapshot: *mut c_void, nkeys: i32, scankeys: *mut u8) -> HeapScanDesc;
    //fn heap_rescan(scan: HeapScanDesc, scankeys: *mut u8);
    fn heap_getnext(scan: HeapScanDesc, direction: i32) -> *const HeapTupleData<'static>;

    fn heap_endscan(scan: HeapScanDesc);

    fn GetTransactionSnapshot() -> *mut c_void;
    //fn ScanKeyInit(entry: *mut u8, attr_num: u16, strat_num: u16, regproc: u32, arg: usize);

    fn heap_deform_tuple(tuple: *const HeapTupleData, desc: *const RawTupleDesc, values: *mut Datum, isnull: *mut bool);
}


pub struct Heap(*const Relation);
pub struct HeapScan<'a> {
    ptr: HeapScanDesc,
    tupledesc: *const RawTupleDesc,
    marker: PhantomData<&'a Heap>,
}

impl Heap {
    pub fn open(oid: Oid) -> Heap {
        unsafe {
            error::convert_postgres_error(|| Heap(heap_open(oid, 1)) )
        }
    }

    pub fn scan(&self) -> HeapScan {
        error::convert_postgres_error(|| {
            unsafe {
                let snap = GetTransactionSnapshot();
                HeapScan {
                    ptr: heap_beginscan(self.0, snap, 0, ptr::null_mut()),
                    tupledesc: (*self.0).td,
                    marker: PhantomData,
                }
            }
        })
    }
}



// TODO: verify in build that C compiler is capable of aligning
#[repr(C)]
#[derive(Clone, Copy)]
struct ItemPointerData {
    foo: [u8; 6]
}


#[repr(C)]
struct HeapTupleHeader {
    pad: [u8; 22],
    //pad: [u8; RELATT_OFFSET],
    t_hoff: u8,
    // tail ...
}

pub struct HeapTuple<'a> {
    data: HeapTupleData<'a>,
    tupledesc: *const RawTupleDesc,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HeapTupleData<'a> {
    t_len: u32,
    t_self: ItemPointerData,
    t_tableOid: Oid,
    t_data: &'a HeapTupleHeader,
}


impl<'a> HeapTuple<'a> {
    pub fn deform(&self) -> Vec<Option<Datum<'a>>> {
        unsafe {
            error::convert_postgres_error(|| {
                let natts = (*self.tupledesc).natts as usize;
                let mut vals: Vec<Datum> = Vec::with_capacity(natts);
                let mut flags: Vec<bool> = Vec::with_capacity(natts);
                vals.set_len(natts);
                flags.set_len(natts);

                heap_deform_tuple(&self.data, self.tupledesc, vals.as_mut_ptr(), flags.as_mut_ptr());

                vals.into_iter().zip(flags).map(|(v, f)| if f { None } else { Some(v) }).collect()
            })
        }
    }
}

impl<'a> Iterator for HeapScan<'a> {
    type Item = HeapTuple<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let tuple = error::convert_postgres_error(|| heap_getnext(self.ptr, 1));
            if tuple.is_null() {
                None
            } else {
                Some(HeapTuple {
                    data: *tuple,
                    tupledesc: self.tupledesc,
                })
            }
        }
    }
}

impl<'a> Drop for HeapScan<'a> {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| heap_endscan(self.ptr))
        }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| relation_close(self.0, 1))
        }
    }
}
/*
pub fn do_index_scan(rel: Oid, idx: Oid) -> i32 {
    let mut counter = 0;
    unsafe {
        let heap = heap_open(rel, 1);
        let index = index_open(idx, 1);

        let btint4cmp = 184;
        let mut keybuf = [0u8; ::LEN_SCANKEYDATA];
        ScanKeyInit(keybuf.as_mut_ptr(), 1, 3, btint4cmp, 4);

        let snap = GetTransactionSnapshot();
        assert!(!snap.is_null());
        let scan = index_beginscan(heap, index, snap, 1, 0);
        index_rescan(scan, keybuf.as_mut_ptr(), 1, ptr::null_mut(), 0);
        loop {
            let thing = index_getnext(scan, 1);
            if thing.is_null() { break; }
            counter += 1;
        }
        index_endscan(scan);

        index_close(index, 1);
        relation_close(heap, 1);
    }
    counter
}
*/
