use std::os::raw::{c_int, c_char};
use std::ffi::CString;
use std::ptr;

use types::oid;
use Datum;
use error;


mod ffi;
mod tuples;
mod parameter;
mod cursor;
pub use self::tuples::{SpiTuples, SpiTuplesIter};
pub use self::parameter::Parameter;
pub use self::cursor::{SpiCursor, Direction};

use self::ffi::*;

pub struct SpiContext {
    _private: (),
}

// you wanted to do this, but it's not allowed in this context
#[derive(Clone, Debug)]
pub enum ExecError {
    Copy,
    Transaction,
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

impl SpiContext {
    pub unsafe fn create() -> SpiContext {
        let ret = SPI_connect();
        assert_eq!(ret, SPI_OK_CONNECT);

        SpiContext { _private: () }
    }


    pub fn execute<'a>(&'a self, sql: &str, args: &[Parameter]) -> Result<SpiResult<'a>, ExecError> {
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

            match ret {
                // success cases:
                SPI_OK_SELECT => Ok(SpiResult::Select(SpiTuples::new())),
                SPI_OK_INSERT_RETURNING => Ok(SpiResult::InsertReturning(SpiTuples::new())),
                SPI_OK_DELETE_RETURNING => Ok(SpiResult::DeleteReturning(SpiTuples::new())),
                SPI_OK_UPDATE_RETURNING => Ok(SpiResult::UpdateReturning(SpiTuples::new())),
                SPI_OK_SELINTO => Ok(SpiResult::SelectInto(SPI_processed)),
                SPI_OK_INSERT => Ok(SpiResult::Insert(SPI_processed)),
                SPI_OK_DELETE => Ok(SpiResult::Delete(SPI_processed)),
                SPI_OK_UPDATE => Ok(SpiResult::Update(SPI_processed)),
                SPI_OK_UTILITY => Ok(SpiResult::Utility),
                SPI_OK_REWRITTEN => Ok(SpiResult::Rewritten),

                // error cases:
                SPI_ERR_ARGUMENT | SPI_ERR_UNCONNECTED => unreachable!(), // we ensure this can't happen
                SPI_ERR_COPY => Err(ExecError::Copy),
                SPI_ERR_TRANSACTION => Err(ExecError::Transaction),
                SPI_ERR_OPUNKNOWN => panic!("PostgreSQL docs state that this should not happen."),

                x => panic!("Unknown SPI_execute return code: {}", x),
            }
        }
    }

    // NB this panics instead of erroring (due to underlying pg impl); perhaps this is not always what we want
    pub fn execute_cursor<'a>(&'a self, sql: &str, args: &[Parameter]) -> SpiCursor<'a> {
        unsafe {
            let ret = error::convert_postgres_error(|| {
                let command = CString::new(sql).unwrap();

                let argtypes: Vec<oid> = args.iter().map(|x| x.oid).collect();
                let values: Vec<Datum> = args.iter().map(|x| x.value.unwrap_or(Datum::create(0))).collect();
                let nulls: Vec<c_char> = args.iter().map(|x| x.value.map(|_| b' ').unwrap_or(b'n') as c_char).collect();

                SPI_cursor_open_with_args(ptr::null(), // null name
                                          command.as_ptr(),
                                          args.len() as c_int,
                                          argtypes.as_ptr(),
                                          values.as_ptr(),
                                          nulls.as_ptr(),
                                          false,
                                          0x0002) // SCROLL
            });

            SpiCursor::new(ret)
        }
    }
}

impl Drop for SpiContext {
    fn drop(&mut self) {
        unsafe {
            error::convert_postgres_error_dtor(|| {
                SPI_finish();
            });
        }
    }
}
