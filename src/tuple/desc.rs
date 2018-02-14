use std::marker::PhantomData;
use std::panic::{UnwindSafe, RefUnwindSafe};

use types::Oid;


#[repr(C, packed)]
pub struct RawTupleDesc {
    pub natts: i32,
    pub tdtypeid: Oid,
    pub tdtypmod: i32,
    pub tdhasoid: u8,
    pub tdrefcount: i32,
    // ...
}

pub struct RcTupleDesc {
    ptr: *const RawTupleDesc,
}


impl Drop for RcTupleDesc {
    fn drop(&mut self) {
        unsafe {
            DecrTupleDescRefCount(self.ptr)
        }
    }
}

#[derive(Clone, Copy)]
pub struct RefTupleDesc<'a> {
    ptr: *const RawTupleDesc,
    marker: PhantomData<&'a ()>,
}

pub unsafe trait TupleDesc: UnwindSafe + RefUnwindSafe {
    fn as_raw(&self) -> *const RawTupleDesc;
    unsafe fn from_raw(ptr: *const RawTupleDesc) -> Self;

    fn num_attributes(&self) -> i32 {
        unsafe { (*self.as_raw()).natts }
    }
}

extern "C" {
    fn IncrTupleDescRefCount(ptr: *const RawTupleDesc);
    fn DecrTupleDescRefCount(ptr: *const RawTupleDesc);
}

unsafe impl TupleDesc for RcTupleDesc {
    fn as_raw(&self) -> *const RawTupleDesc {
        self.ptr
    }

    unsafe fn from_raw(ptr: *const RawTupleDesc) -> RcTupleDesc {
        assert!((*ptr).tdrefcount >= 0);
        IncrTupleDescRefCount(ptr);
        RcTupleDesc { ptr }
    }
}

unsafe impl<'a> TupleDesc for RefTupleDesc<'a> {
    fn as_raw(&self) -> *const RawTupleDesc {
        self.ptr
    }

    unsafe fn from_raw(ptr: *const RawTupleDesc) -> RefTupleDesc<'a> {
        RefTupleDesc {
            ptr,
            marker: PhantomData,
        }
    }
}
