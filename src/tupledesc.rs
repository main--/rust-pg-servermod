use std::marker::PhantomData;

use types::Oid;


#[repr(C, packed)]
pub struct RawTupleDesc {
    pub natts: i32,
    pub tdtypeid: Oid,
    // ...
}

pub struct RcTupleDesc {
    ptr: *const RawTupleDesc,
}

#[derive(Clone, Copy)]
pub struct RefTupleDesc<'a> {
    ptr: *const RawTupleDesc,
    marker: PhantomData<&'a ()>,
}

pub unsafe trait TupleDesc {
    fn as_raw(&self) -> *const RawTupleDesc;
    unsafe fn from_raw(ptr: *const RawTupleDesc) -> Self;

    fn num_attributes(&self) -> i32 {
        unsafe { (*self.as_raw()).natts }
    }
}

unsafe impl TupleDesc for RcTupleDesc {
    fn as_raw(&self) -> *const RawTupleDesc {
        self.ptr
    }

    unsafe fn from_raw(ptr: *const RawTupleDesc) -> RcTupleDesc {
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
