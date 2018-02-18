use std::os::raw::{c_void, c_long};
use std::marker::PhantomData;

use error;
use super::{SpiContext, SpiTuples};
use super::ffi::*;

pub struct SpiCursor<'a> {
    ptr: *mut c_void,
    marker: PhantomData<&'a SpiContext>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

impl<'a> SpiCursor<'a> {
    pub(super) unsafe fn new(ptr: *mut c_void) -> SpiCursor<'a> {
        SpiCursor {
            ptr,
            marker: PhantomData,
        }
    }

    pub fn fetch(&mut self, direction: Direction, count: u64) -> SpiTuples {
        assert_eq!(count as c_long as u64, count); // overflow check

        unsafe {
            error::convert_postgres_error(|| SPI_cursor_fetch(self.ptr, direction == Direction::Forward, count as c_long));
            SpiTuples::new()
        }
    }

    pub fn move_absolute(&mut self, index: u64) {
        assert_eq!(index as c_long as u64, index); // overflow check

        error::convert_postgres_error(|| unsafe { SPI_scroll_cursor_move(self.ptr, 2, index as c_long) });
    }

    pub fn move_relative(&mut self, offset: i64) {
        assert_eq!(offset as c_long as i64, offset); // overflow check

        error::convert_postgres_error(|| unsafe { SPI_scroll_cursor_move(self.ptr, 3, offset as c_long) });
    }
}

impl<'a> Drop for SpiCursor<'a> {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| SPI_cursor_close(self.ptr))
        }
    }
}
