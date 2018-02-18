use std::os::raw::{c_int, c_char, c_void, c_long};

use Datum;
use types::oid;
use tuple::desc::RawTupleDesc;

#[repr(C)]
pub struct SPITupleTable {
    pub tuptabcxt: *mut c_void,
    pub alloced: u64,
    pub free: u64,
    pub tupdesc: *mut RawTupleDesc,
    pub vals: *mut *mut c_void,
    // ...
}

extern "C" {
    pub fn SPI_connect() -> c_int;
    pub fn SPI_finish() -> c_int;
    pub fn SPI_execute_with_args(command: *const c_char,
                                 nargs: c_int,
                                 argtypes: *const oid,
                                 values: *const Datum,
                                 nulls: *const c_char,
                                 read_only: bool,
                                 count: c_long) -> c_int;
    pub fn SPI_cursor_open_with_args(name: *const c_char,
                                     command: *const c_char,
                                     nargs: c_int,
                                     argtypes: *const oid,
                                     values: *const Datum,
                                     nulls: *const c_char,
                                     read_only: bool,
                                     cursor_options: c_int) -> *mut c_void;
    pub fn SPI_cursor_fetch(cursor: *mut c_void, forward: bool, count: c_long);
    pub fn SPI_scroll_cursor_move(cursor: *mut c_void, direction: i32, count: c_long);
    pub fn SPI_cursor_close(cursor: *mut c_void);

    pub fn SPI_freetuptable(tuptable: *mut SPITupleTable);
    pub static mut SPI_tuptable: *mut SPITupleTable;
    pub static mut SPI_processed: u64;
}

pub const SPI_OK_CONNECT: i32 = 1;
// pub const SPI_OK_FINISH: i32 = 2;

// pub const SPI_OK_FETCH: i32 = 3;

pub const SPI_OK_UTILITY: i32 = 4;
pub const SPI_OK_SELECT: i32 = 5;
pub const SPI_OK_SELINTO: i32 = 6;
pub const SPI_OK_INSERT: i32 = 7;
pub const SPI_OK_DELETE: i32 = 8;
pub const SPI_OK_UPDATE: i32 = 9;
// pub const SPI_OK_CURSOR: i32 = 10;
pub const SPI_OK_INSERT_RETURNING: i32 = 11;
pub const SPI_OK_DELETE_RETURNING: i32 = 12;
pub const SPI_OK_UPDATE_RETURNING: i32 = 13;
pub const SPI_OK_REWRITTEN: i32 = 14;

// pub const SPI_ERR_CONNECT: i32 = -1;
pub const SPI_ERR_COPY: i32 = -2;
pub const SPI_ERR_OPUNKNOWN: i32 = -3;
pub const SPI_ERR_UNCONNECTED: i32 = -4;
// pub const SPI_ERR_CURSOR: i32 = -5;
pub const SPI_ERR_ARGUMENT: i32 = -6;
// pub const SPI_ERR_PARAM: i32 = -7;
pub const SPI_ERR_TRANSACTION: i32 = -8;
// pub const SPI_ERR_NO_ATTRIBUTE: i32 = -9;
// pub const SPI_ERR_NO_OUTFUNC: i32 = -10;
// pub const SPI_ERR_TYP_UNKNOWN: i32 = -11;
// pub const SPI_ERR_REL_DUPLICATE: i32 = -12;
// pub const SPI_ERR_REL_NOT_FOUND: i32 = -13;
