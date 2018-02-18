use std::os::raw::c_void;
use std::marker::PhantomData;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::slice;

use tuple::desc::{TupleDesc, RefTupleDesc};
use tuple::slot::{TupleSlot, SlottedTuple};
use alloc::MemoryContext;

use error;
use super::SpiContext;
use super::ffi::*;

pub struct SpiTuples<'a> {
    table: *mut SPITupleTable,
    count: u64,
    marker: PhantomData<&'a SpiContext>,
}

impl<'a> SpiTuples<'a> {
    pub(super) unsafe fn new() -> SpiTuples<'a> {
        SpiTuples {
            table: SPI_tuptable,
            count: SPI_processed,
            marker: PhantomData,
        }
    }

    pub fn iter<'b, 'alloc>(&'b self, alloc: &'alloc MemoryContext<'alloc>) -> SpiTuplesIter<'a, 'b, 'alloc> {
        SpiTuplesIter {
            tuples: self,
            index: 0,
            slot: TupleSlot::create(self.tuple_desc(), alloc),
        }
    }

    pub fn tuple_desc<'b>(&'b self) -> RefTupleDesc<'b> {
        unsafe {
            RefTupleDesc::from_raw((*self.table).tupdesc)
        }
    }
}

pub struct SpiTuplesIter<'a: 'b, 'b, 'alloc> {
    tuples: &'b SpiTuples<'a>,
    index: usize,
    slot: TupleSlot<'alloc, RefTupleDesc<'b>>,
}

impl<'a: 'b, 'b, 'alloc> SpiTuplesIter<'a, 'b, 'alloc> {
    pub fn next<'c>(&'c mut self) -> Option<SlottedTuple<'alloc, 'c, 'b, RefTupleDesc<'b>>> {
        unsafe {
            let tuples: &'static [*mut c_void] = slice::from_raw_parts((*self.tuples.table).vals, self.tuples.count as usize);

            tuples.get(self.index).map(move |&tuple| {
                assert!(!tuple.is_null());
                self.index += 1;

                self.slot.store_tuple(tuple, 0) // InvalidBuffer
            })
        }
    }
}

impl<'a> Debug for SpiTuples<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "SPI[")?;

        // quickly create a new allocator, who even cares
        let mctx = MemoryContext::create_allocset(None, 0, 8192, 8192 * 1024);
        let mut iter = self.iter(&mctx);
        while let Some(x) = iter.next() {
            write!(fmt, "{:?}, ", x)?;
        }
        write!(fmt, "]")?;
        Ok(())
    }
}

impl<'a> Drop for SpiTuples<'a> {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| SPI_freetuptable(self.table))
        }
    }
}
