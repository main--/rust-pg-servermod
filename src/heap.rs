use std::os::raw::c_void;
use std::marker::PhantomData;
use std::ptr;

use alloc::MemoryContext;
use types::Oid;
use error;
use relation::{Relation, GetTransactionSnapshot};
use tupledesc::{TupleDesc, RefTupleDesc};
use tupleslot::{TupleSlot, SlottedTuple};

#[repr(C)]
struct HeapScanDescData {
    _pad: [u8; ::RS_CBUF_OFFSET],
    rs_cbuf: i32,
}
type HeapScanDesc = *mut HeapScanDescData;

type HeapTupleData = c_void;
extern "C" {
    fn heap_open(relation: Oid, lockmode: i32) -> *const Relation;
    fn relation_close(relation: *const Relation, lockmode: i32);

    fn heap_beginscan(relation: *const Relation, snapshot: *mut c_void, nkeys: i32, scankeys: *mut u8) -> HeapScanDesc;
    //fn heap_rescan(scan: HeapScanDesc, scankeys: *mut u8);
    fn heap_getnext(scan: HeapScanDesc, direction: i32) -> *const HeapTupleData;
    fn heap_endscan(scan: HeapScanDesc);

    //fn heap_deform_tuple(tuple: *const HeapTupleData, desc: *const RawTupleDesc, values: *mut Datum, isnull: *mut bool);
}



// FIXME private member
pub struct Heap(pub(crate) *const Relation);
pub struct RawHeapScan<'a> {
    ptr: HeapScanDesc,
    marker: PhantomData<&'a Heap>,
}

pub struct HeapScan<'alloc, 'h> {
    raw: RawHeapScan<'h>,
    slot: TupleSlot<'alloc, RefTupleDesc<'h>>,
}

impl Heap {
    pub fn open(oid: Oid) -> Heap {
        unsafe {
            error::convert_postgres_error(|| Heap(heap_open(oid, 1)) )
        }
    }

    pub fn scan<'alloc, 'h>(&'h self, alloc: &'alloc MemoryContext<'alloc>) -> HeapScan<'alloc, 'h> {
        HeapScan {
            raw: self.scan_raw(),
            slot: TupleSlot::create(self.tuple_desc(), alloc),
        }
    }

    pub fn scan_raw<'a>(&'a self) -> RawHeapScan<'a> {
        error::convert_postgres_error(|| {
            unsafe {
                let snap = GetTransactionSnapshot();
                RawHeapScan {
                    ptr: heap_beginscan(self.0, snap, 0, ptr::null_mut()),
                    marker: PhantomData,
                }
            }
        })
    }

    pub fn tuple_desc<'a>(&'a self) -> RefTupleDesc<'a> {
        unsafe {
            RefTupleDesc::from_raw((*self.0).td)
        }
    }
}

impl<'alloc, 'h> HeapScan<'alloc, 'h> {
    pub fn next<'a>(&'a mut self) -> Option<SlottedTuple<'alloc, 'a, 'h, RefTupleDesc<'h>>> {
        unsafe {
            match self.raw.next() {
                None => None,
                Some(tuple) => {
                    let buffer = (*self.raw.ptr).rs_cbuf;
                    Some(self.slot.store_tuple(tuple, buffer))
                }
            }
        }
    }
}

impl<'a> Iterator for RawHeapScan<'a> {
    type Item = *const HeapTupleData;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let tuple = error::convert_postgres_error(|| heap_getnext(self.ptr, 1));
            if tuple.is_null() {
                None
            } else {
                Some(tuple)
            }
        }
    }
}

impl<'a> Drop for RawHeapScan<'a> {
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
