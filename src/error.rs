pub extern crate unreachable;

use std::os::raw::c_char;
use std::{ptr, mem, thread};
use std::ffi::CString;
use std::panic::{self, UnwindSafe};
use std::error::Error;
use std::fmt::{Debug, Display, Result as FmtResult, Formatter};
use std::any::Any;
use std::sync::atomic::{AtomicPtr, Ordering};

use alloc::MemoryContext;

extern "C" {
    fn errstart(level: i32, filename: *const c_char, line: i32, funcname: *const c_char, domain: *const c_char) -> u32;
    fn errmsg(fmt: *const c_char, ...);
    fn errfinish(dummy: i32, ...);
}

static RUST_PANIC_FUNCNAME: [u8; 11] = *b"RUST PANIC\0";
fn rust_panic_funcname_ptr() -> *const c_char { RUST_PANIC_FUNCNAME.as_ptr() as *const c_char }

// postgres is single-threaded software.
static LAST_RUST_PANIC: AtomicPtr<Box<Any + Send>> = AtomicPtr::new(ptr::null_mut());

// this is a formality as it can't ever happen
// in theory it would avoid a potential memory leak if you unload and re-load us a lot of times
#[doc(hidden)]
pub unsafe extern "C" fn _PG_fini() {
    let ptr = LAST_RUST_PANIC.load(Ordering::Relaxed);
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

#[inline(always)]
pub fn convert_rust_panic<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> R {
    panic::catch_unwind(f).unwrap_or_else(|r| convert_rust_panic_inner(r))
}

#[inline(never)]
fn convert_rust_panic_inner(e: Box<Any + Send>) -> ! {
    unsafe {
        let e = match e.downcast::<PgError>() {
            Ok(pge) => pge.rethrow(), // this was a postgres error to begin with, just rethrow that
            Err(e) => e,
        };

        if errstart(20, ptr::null(), 0, rust_panic_funcname_ptr(), ptr::null()) != 0 {
            // TODO: errcode or some shit
            {
                let text = e.downcast_ref::<&str>().cloned().or_else(|| e.downcast_ref::<String>().map(|s| &s[..])).unwrap_or("<no text>");
                match CString::new(text) {
                    Ok(text_cs) => errmsg(b"%s\0" as *const _ as *const _, text_cs.as_ptr()),
                    Err(_) => errmsg(b"<string conversion error>\0" as *const _ as *const _),
                }
            }

            // replace LAST_RUST_PANIC
            let new_panic = Box::into_raw(Box::new(e));
            let old_panic = LAST_RUST_PANIC.swap(new_panic, Ordering::Relaxed);
            if !old_panic.is_null() {
                drop(Box::from_raw(old_panic));
            }

            errfinish(0);
        }
        self::unreachable::unreachable()
    }
}


extern "C" {
    static mut PG_exception_stack: *mut u8;
    static mut error_context_stack: *mut u8;
    fn sigsetjmp(env: *mut u8, savesigs: i32) -> i32;

    fn FlushErrorState();
    fn CopyErrorData() -> *mut ErrorData;
    // fn FreeErrorData(ed: *mut ErrorData);
    fn ReThrowError(ed: *mut ErrorData) -> !;
    fn pg_re_throw() -> !;
}

#[repr(C)]
#[derive(Debug)]
struct ErrorData {
    elevel: i32,
    output_to_server: u8,
    output_to_client: u8,
    show_funcname: u8,
    hide_stmt: u8,
    hide_ctx: u8,
    filename: *const c_char,
    lineno: i32,
    funcname: *const c_char,
    domain: *const c_char,
    context_domain: *const c_char,
    sqlerrcode: i32,
    message: *const c_char,
    detail: *const c_char,
    detail_log: *const c_char,
    hint: *const c_char,
    context: *const c_char,
    schema_name: *const c_char,
    table_name: *const c_char,
    column_name: *const c_char,
    datatype_name: *const c_char,
    constraint_name: *const c_char,
    cursorpos: i32,
    internalpos: i32,
    internalquery: *const c_char,
    saved_errno: i32,

    assoc_context: MemoryContext<'static>, // imperfect approximation
}

// TODO: error builder api so I can construct one myself and pass it to panic!()
// this is mostly relevant so I can report my own sql-compatible errors

pub struct PgError(*mut ErrorData);
unsafe impl Send for PgError {}
impl PgError {
    pub unsafe fn rethrow(self) -> ! {
        // bending postgres memory allocation to rust is hard
        // the (admittedly /very/ awkward) solution here is
        // to throw, catch, free our copy, then immediately rethrow

        let save_exception_stack = PG_exception_stack;
        let save_context_stack = error_context_stack;
        let mut jmpbuf = [0u8; ::LEN_SIGJMPBUF];

        if sigsetjmp(jmpbuf.as_mut_ptr(), 0) == 0 {
            PG_exception_stack = jmpbuf.as_mut_ptr();
            ReThrowError(self.0)
            // control flows into the else block from here
        } else {
            PG_exception_stack = save_exception_stack;
            error_context_stack = save_context_stack;

            // error is on the PG error stack, time to free our copy
            drop(self);
            pg_re_throw();
            // unreachable
        }
    }
}
impl Debug for PgError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        Debug::fmt(unsafe { &*self.0 }, fmt)
    }
}
impl Display for PgError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "lul")
    }
}
impl Error for PgError {
    fn description(&self) -> &str { "Postgres error" }
}

#[inline]
pub fn convert_postgres_error<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> R {
    catch_postgres_error(f).unwrap_or_else(|e| convert_postgres_error_inner(e))
}
#[inline]
pub fn convert_postgres_error_dtor<F: FnOnce() + UnwindSafe>(f: F) {
    catch_postgres_error(f).unwrap_or_else(|e| {
        // this guard is critical to avoid double panics (we /really/ don't want to abort a backend)
        if thread::panicking() {
            return;
        }

        convert_postgres_error_inner(e)
    })
}

#[inline(never)]
fn convert_postgres_error_inner(e: PgError) -> ! {
    unsafe {
        // check ptr equality for our magic funcname
        if (*e.0).funcname == rust_panic_funcname_ptr() {
            let ptr = LAST_RUST_PANIC.swap(ptr::null_mut(), Ordering::Relaxed);
            assert!(!ptr.is_null());
            let payload = Box::from_raw(ptr);

            panic::resume_unwind(*payload)
        } else {
            // just throw as PgError
            panic!(e)
        }
    }
}

#[inline]
pub fn catch_postgres_error<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Result<R, PgError> {
    unsafe {
        let restore = RestorePgExceptionStack::capture();
        let mut jmpbuf: [u8; ::LEN_SIGJMPBUF] = mem::uninitialized();

        if sigsetjmp(jmpbuf.as_mut_ptr(), 0) == 0 {
            PG_exception_stack = jmpbuf.as_mut_ptr();

            let ret = f();
            drop(restore);
            Ok(ret)
        } else {
            drop(restore);

            Err(record_pg_error())
        }
    }
}

// this exists so we reset the exception stack *even* if a rust panic is unwinding
// through our postgres error handler
//
// this allows you to panic!() inside a catch_postgres_error block without
// completely messing up the stack (once we convert the rust panic to a pg error,
// we would try to jump to this catch block ... which has long since returned!)
struct RestorePgExceptionStack {
    save_exception_stack: *mut u8,
    save_context_stack: *mut u8,
}

impl RestorePgExceptionStack {
    fn capture() -> RestorePgExceptionStack {
        unsafe {
            RestorePgExceptionStack {
                save_exception_stack: PG_exception_stack,
                save_context_stack: error_context_stack,
            }
        }
    }
}

impl Drop for RestorePgExceptionStack {
    fn drop(&mut self) {
        unsafe {
            PG_exception_stack = self.save_exception_stack;
            error_context_stack = self.save_context_stack;
        }
    }
}

#[inline(never)]
unsafe fn record_pg_error() -> PgError {
    // the hard part: catching an error
    // because the concept of a "current memory context"
    // is hard to encapsulate in Rust (esp. when we want to unwind down the stack),
    // we create a new memory context just for this error data

    let mctx = MemoryContext::create_allocset(None, 0, 8192, 8192 * 1024);
    mctx.set_current();
    mem::forget(mctx);

    let err = PgError(CopyErrorData());
    FlushErrorState();
    err
}
