use std::marker::PhantomData;

use relation::RawTupleDesc;

pub struct RcTupleDesc {
    ptr: *const RawTupleDesc,
}

#[derive(Clone, Copy)]
pub struct RefTupleDesc<'a> {
    ptr: *const RawTupleDesc,
    marker: PhantomData<&'a ()>,
}

pub trait TupleDesc {
    fn as_raw(&self) -> *const RawTupleDesc;
    unsafe fn from_raw(ptr: *const RawTupleDesc) -> Self;

}

impl TupleDesc for RcTupleDesc {
    fn as_raw(&self) -> *const RawTupleDesc {
        self.ptr
    }

    unsafe fn from_raw(ptr: *const RawTupleDesc) -> RcTupleDesc {
        RcTupleDesc { ptr }
    }
}
impl<'a> TupleDesc for RefTupleDesc<'a> {
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
