use std::marker::PhantomData;
use std::os::raw::c_void;
use std::cell::Cell;
use std::fmt::{Result as FmtResult, Formatter, Debug};
use std::panic::AssertUnwindSafe;

use error;
use Datum;
use alloc::MemoryContext;
use super::desc::{TupleDesc, RefTupleDesc, RawTupleDesc};

#[repr(C)]
struct RawTupleSlot {
    _pad: [u8; 16],
    tts_tupleDescriptor: *const RawTupleDesc,
}

pub struct TupleSlot<'alloc, T: TupleDesc> {
    ptr: *mut RawTupleSlot,
    td: PhantomData<T>,
    memory: PhantomData<&'alloc MemoryContext<'alloc>>,
}
pub struct SlottedTuple<'alloc: 'slot, 'slot, 'tuple, T: TupleDesc + 'slot> {
    slot: &'slot mut TupleSlot<'alloc, T>,
    tuple: PhantomData<AssertUnwindSafe<Cell<&'tuple ()>>>,
}

extern "C" {
    fn MakeSingleTupleTableSlot(tupdesc: *const RawTupleDesc) -> *mut RawTupleSlot;
    fn ExecDropSingleTupleTableSlot(slot: *mut RawTupleSlot);
    fn ExecStoreTuple(tuple: *const c_void, slot: *mut RawTupleSlot, buffer: i32, should_free: bool) -> *mut c_void;
    //fn slot_getallattrs(slot: *mut c_void);
    fn slot_getattr(slot: *mut RawTupleSlot, attnum: i32, isnull: *mut bool) -> Datum<'static>;
}

impl<'a, T: TupleDesc> TupleSlot<'a, T> {
    // allocates, this is expensive
    pub fn create(tupledesc: T, allocator: &'a MemoryContext<'a>) -> TupleSlot<'a, T> {
        unsafe {
            allocator.set_current();
            let ptr = MakeSingleTupleTableSlot(tupledesc.as_raw());
            TupleSlot {
                ptr,
                td: PhantomData,
                memory: PhantomData,
            }
        }
    }

    pub unsafe fn store_tuple<'slot, 'tuple>(&'slot mut self,
                                             tuple: *const c_void,
                                             buffer: i32) -> SlottedTuple<'a, 'slot, 'tuple, T> {
        ExecStoreTuple(tuple, self.ptr, buffer, false);
        self.filled()
    }

    pub unsafe fn filled<'slot, 'tuple>(&'slot mut self) -> SlottedTuple<'a, 'slot, 'tuple, T> {
        SlottedTuple {
            slot: self,
            tuple: PhantomData,
        }
    }


    // NB we COULD build a T out of this but in the Rc case that's usually not what you need, so we avoid the overhead
    pub fn tupledesc<'slot>(&'slot self) -> RefTupleDesc<'slot> {
        unsafe {
            RefTupleDesc::from_raw((*self.ptr).tts_tupleDescriptor)
        }
    }
}

impl<'a, T: TupleDesc> Drop for TupleSlot<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ExecDropSingleTupleTableSlot(self.ptr)
        }
    }
}

impl<'alloc, 'slot, 'tuple, T: TupleDesc + 'slot> SlottedTuple<'alloc, 'slot, 'tuple, T> {
    pub fn slot(&'slot self) -> &'slot TupleSlot<'alloc, T> { self.slot }

    pub fn attribute<'a>(&'a self, attr: i32) -> Option<Datum<'a>> {
        unsafe {
            error::convert_postgres_error(|| {
                let mut isnull = false;
                let value = slot_getattr(self.slot.ptr, attr, &mut isnull);
                if isnull {
                    None
                } else {
                    Some(value)
                }
            })
        }
    }
}

impl<'alloc, 'slot, 'tuple, T: TupleDesc + 'slot> Debug for SlottedTuple<'alloc, 'slot, 'tuple, T> {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "SLOT[")?;

        let tupledesc = self.slot().tupledesc();
        for i in 0..tupledesc.num_attributes() {
            write!(fmt, "{:?}, ", self.attribute(i + 1))?;
        }
        write!(fmt, "]")?;

        Ok(())
    }
}
