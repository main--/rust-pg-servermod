use std::os::raw::{c_int, c_char, c_void, c_long};
use std::marker::PhantomData;
use std::ffi::CString;
use std::slice;

use types::{oid, StaticallyTyped};
use Datum;
use error;
use tuple::desc::{TupleDesc, RefTupleDesc, RawTupleDesc};
use tuple::slot::{TupleSlot, SlottedTuple};
use alloc::MemoryContext;

#[repr(C)]
struct SPITupleTable {
    tuptabcxt: *mut c_void,
    alloced: u64,
    free: u64,
    tupdesc: *mut RawTupleDesc,
    vals: *mut *mut c_void,
    // ...
}

extern "C" {
    fn SPI_connect() -> c_int;
    fn SPI_finish() -> c_int;
    fn SPI_execute_with_args(command: *const c_char,
                             nargs: c_int,
                             argtypes: *const oid,
                             values: *const Datum,
                             nulls: *const c_char,
                             read_only: bool,
                             count: c_long) -> c_int;
    fn SPI_cursor_open_with_args(name: *const c_char,
                                 command: *const c_char,
                                 nargs: c_int,
                                 argtypes: *const oid,
                                 values: *const Datum,
                                 nulls: *const c_char,
                                 read_only: bool,
                                 cursor_options: c_int) -> *mut c_void;
    fn SPI_cursor_close(cursor: *mut c_void);

    fn SPI_freetuptable(tuptable: *mut SPITupleTable);
    static mut SPI_tuptable: *mut SPITupleTable;
    static mut SPI_processed: u64;
}


pub struct SpiContext {
    _private: (),
}

#[repr(i32)]
#[allow(dead_code)]
enum SpiRet {
    OkConnect = 1,
    OkFinish = 2,

    OkFetch = 3,

    OkUtility = 4,
    OkSelect = 5,
    OkSelInto = 6,
    OkInsert = 7,
    OkDelete = 8,
    OkUpdate = 9,
    OkCursor = 10,
    OkInsertReturning = 11,
    OkDeleteReturning = 12,
    OkUpdateReturning = 13,
    OkRewritten = 14,

    OkRelRegister = 15,
    OkRelUnregister = 16,
    OkTdRegister = 17,

    ErrConnect = -1,
    ErrCopy = -2,
    ErrOpUnknown = -3,
    ErrUnconnected = -4,
    ErrCursor = -5,
    ErrArgument = -6,
    ErrParam = -7,
    ErrTransaction = -8,
    ErrNoAttribute = -9,
    ErrNoOutFunc = -10,
    ErrTypUnknown = -11,
    ErrRelDuplicate = -12,
    ErrRelNotFound = -13,
}

// you wanted to do this, but it's not allowed in this context
#[derive(Clone, Debug)]
pub enum ExecError {
    Copy,
    Transaction,
}

impl SpiContext {
    pub unsafe fn create() -> SpiContext {
        let ret = SPI_connect();
        assert_eq!(ret, SpiRet::OkConnect as i32);

        SpiContext { _private: () }
    }


    pub fn execute<'a>(&'a self, sql: &str, args: &[QueryParameter]) -> Result<SpiResult<'a>, ExecError> {
        unsafe {
            let ret = error::convert_postgres_error(|| {
                let command = CString::new(sql).unwrap();

                let argtypes: Vec<oid> = args.iter().map(|x| x.oid).collect();
                let values: Vec<Datum> = args.iter().map(|x| x.value.unwrap_or(Datum::create(0))).collect();
                let nulls: Vec<c_char> = args.iter().map(|x| x.value.map(|_| b' ').unwrap_or(b'n') as c_char).collect();

                SPI_execute_with_args(command.as_ptr(),
                                      args.len() as c_int,
                                      argtypes.as_ptr(),
                                      values.as_ptr(),
                                      nulls.as_ptr(),
                                      false,
                                      0)
            });

            Ok(if ret == SpiRet::OkSelect as i32 {
                SpiResult::Select(SpiTuples::new())
            } else if ret == SpiRet::OkInsertReturning as i32 {
                SpiResult::InsertReturning(SpiTuples::new())
            } else if ret == SpiRet::OkDeleteReturning as i32 {
                SpiResult::DeleteReturning(SpiTuples::new())
            } else if ret == SpiRet::OkUpdateReturning as i32 {
                SpiResult::UpdateReturning(SpiTuples::new())
            } else if ret == SpiRet::OkSelInto as i32 {
                SpiResult::SelectInto(SPI_processed)
            } else if ret == SpiRet::OkInsert as i32 {
                SpiResult::Insert(SPI_processed)
            } else if ret == SpiRet::OkDelete as i32 {
                SpiResult::Delete(SPI_processed)
            } else if ret == SpiRet::OkUpdate as i32 {
                SpiResult::Update(SPI_processed)
            } else if ret == SpiRet::OkUtility as i32 {
                SpiResult::Utility
            } else if ret == SpiRet::OkRewritten as i32 {
                SpiResult::Rewritten
            } else {
                // error cases
                if ret == SpiRet::ErrArgument as i32 {
                    unreachable!(); // we ensure this can't happen
                } else if ret == SpiRet::ErrCopy as i32 {
                    return Err(ExecError::Copy);
                } else if ret == SpiRet::ErrTransaction as i32 {
                    return Err(ExecError::Transaction);
                } else if ret == SpiRet::ErrOpUnknown as i32 {
                    panic!("PostgreSQL docs state that this should not happen.");
                } else if ret == SpiRet::ErrUnconnected as i32 {
                    unreachable!(); // we ensure this can't happen either
                } else {
                    panic!("Unknown SPI_execute return code: {}", ret);
                }
            })
        }
    }
}

impl Drop for SpiContext {
    fn drop(&mut self) {
        unsafe {
            SPI_finish();
        }
    }
}


pub struct QueryParameter<'a> {
    oid: oid,
    value: Option<Datum<'a>>,
}

impl<'a> QueryParameter<'a> {
    pub fn null<T: StaticallyTyped>() -> QueryParameter<'a> {
        QueryParameter {
            oid: T::OID,
            value: None,
        }
    }
}

impl<'a, T: StaticallyTyped + Into<Datum<'a>>> From<T> for QueryParameter<'a> {
    fn from(t: T) -> QueryParameter<'a> {
        QueryParameter {
            oid: T::OID,
            value: Some(t.into()),
        }
    }
}



#[derive(Debug)]
pub enum SpiResult<'a> {
    Select(SpiTuples<'a>),
    SelectInto(u64),
    Insert(u64),
    Delete(u64),
    Update(u64),
    InsertReturning(SpiTuples<'a>),
    DeleteReturning(SpiTuples<'a>),
    UpdateReturning(SpiTuples<'a>),
    Utility,
    Rewritten,
}

pub struct SpiTuples<'a> {
    table: *mut SPITupleTable,
    count: u64,
    marker: PhantomData<&'a SpiContext>,
}

impl<'a> SpiTuples<'a> {
    unsafe fn new() -> SpiTuples<'a> {
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

use std::fmt::{Debug, Formatter, Result as FmtResult};
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
            SPI_freetuptable(self.table);
        }
    }
}
