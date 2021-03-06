use std::os::raw::c_void;
use std::mem::{self, ManuallyDrop};

use Datum;
use catalog;
use alloc::{self, MemoryContext};
use types::{StaticallyTyped, FromDatum, Oid, bytea};
use varlena::Toasted;
use spi::SpiContext;

extern "C" {
    pub fn get_fn_expr_argtype(flinfo: *mut FmgrInfo, argnum: i32) -> Oid;
    pub fn get_fn_expr_rettype(flinfo: *mut FmgrInfo) -> Oid;
}

#[repr(C)]
pub struct Pg_finfo_record {
    pub version: i32,
}


#[repr(C)]
pub struct FmgrInfo {
    fn_addr: *mut c_void,
    fn_oid: Oid,
    fn_nargs: i16,
    fn_strict: u8,
    fn_retset: u8,
    fn_stats: u8,
    fn_extra: *mut c_void,
    fn_mcxt: *mut c_void, // MemoryContextData*
    fn_expr: *mut c_void, // fmNodePtr
}

#[repr(C)]
pub struct FunctionCallInfoData<'a> {
    flinfo: *mut FmgrInfo,
    context: *mut c_void, // fmNodePtr
    resultinfo: *mut c_void, // fmNodePtr
    fncollation: Oid,
    isnull: u8, // bool
    nargs: i16,
    args: [Datum<'a>; super::FUNC_MAX_ARGS],
    argnull: [u8; super::FUNC_MAX_ARGS],
}



// fixme: should never be holding this by value anyways
#[repr(C)]
pub struct FunctionCallInfo<'a>(*mut FunctionCallInfoData<'a>);

impl<'a> FunctionCallInfo<'a> {
    #[inline(always)]
    pub fn oid(&self) -> Oid {
        unsafe {
            (*(*self.0).flinfo).fn_oid
        }
    }

    pub fn return_type(&self) -> Oid {
        unsafe {
            get_fn_expr_rettype((*self.0).flinfo)
        }
    }

    pub fn arg_types<'b>(&'b self) -> ArgTypesIter<'a, 'b> {
        ArgTypesIter {
            fcinfo: self,
            i: 0,
        }
    }

    #[inline(always)]
    pub fn args_strict(&self) -> &[Datum<'a>] {
        unsafe {
            let len = (*self.0).nargs as usize;
            &(*self.0).args[..len]
        }
    }

    // TODO: perhaps return errors for typechecking and nonexistant parameters
    // (instead of panicking)
    pub fn arg<T: StaticallyTyped + FromDatum<'a>>(&self, n: usize) -> Option<T> {
        let typ = self.arg_types().nth(n).expect("missing parameter");
        if T::OID != typ {
            panic!("Argument #{} has type {} but should be {}", n,
                   catalog::get_type_name(typ).unwrap(),
                   catalog::get_type_name(T::OID).unwrap());
        }

        unsafe {
            if (*self.0).argnull[n] == 0 {
                Some(T::from(self.args_strict()[n]))
            } else {
                None
            }
        }
    }

    #[inline(always)]
    pub fn args<'b>(&'b self) -> ArgsIter<'a, 'b> {
        unsafe {
            ArgsIter(self.args_strict().iter().zip((*self.0).argnull.iter()))
        }
    }

    #[inline(always)]
    pub fn return_null(&self) -> Datum<'a> {
        unsafe {
            (*self.0).isnull = 1;
            Datum::create(0)
        }
    }

    #[inline(never)]
    pub fn typecheck(&self, ret_type: Oid, expected_types: &'static [Oid]) {
        // TODO: also print function name in error messages

        // 1. check return type
        if self.return_type() != ret_type {
            panic!("Return type is {} but should be {}",
                   catalog::get_type_name(self.return_type()).unwrap(),
                   catalog::get_type_name(ret_type).unwrap());
        }

        // 2. check arg types
        let mut arg_types = self.arg_types();
        for (i, &expected) in expected_types.iter().enumerate() {
            let actual = arg_types.next().unwrap_or_else(||
                    panic!("Argument #{} is missing", i));
            if actual != expected {
                panic!("Argument {} has type {} but should be {}", i,
                       catalog::get_type_name(actual).unwrap(),
                       catalog::get_type_name(expected).unwrap());
            }
        }

        // 3. no excess arguments
        assert_eq!(arg_types.next(), None, "Function declared with too many arguments");
    }


    pub unsafe fn bootstrap(self) -> FunctionCallContext<'a> {
        FunctionCallContext {
            fcinfo: self,
            allocator: alloc::get_current_ctx(),
            //spi: None,
            has_spi: AssertUnwindSafe(Cell::new(false)),
        }
    }
}

pub struct ArgTypesIter<'a: 'b, 'b> {
    fcinfo: &'b FunctionCallInfo<'a>,
    i: i16,
}
impl<'a: 'b, 'b> Iterator for ArgTypesIter<'a, 'b> {
    type Item = Oid;

    fn next(&mut self) -> Option<Oid> {
        unsafe {
            let flinfo = (*self.fcinfo.0).flinfo;
            if self.i >= (*flinfo).fn_nargs {
                None
            } else {
                let ret = get_fn_expr_argtype(flinfo, self.i as i32);
                self.i += 1;
                Some(ret)
            }
        }
    }

    fn nth(&mut self, n: usize) -> Option<Oid> {
        // sanity; also it's a poor man's overflow check
        assert!(n < ::FUNC_MAX_ARGS);

        self.i += n as i16;
        self.next()
    }
}
impl<'a: 'b, 'b> ExactSizeIterator for ArgTypesIter<'a, 'b> {
    fn len(&self) -> usize {
        unsafe {
            ((*(*self.fcinfo.0).flinfo).fn_nargs - self.i) as usize
        }
    }
}


use std::iter::Zip;
use std::slice::Iter;
pub struct ArgsIter<'a: 'b, 'b>(Zip<Iter<'b, Datum<'a>>, Iter<'b, u8>>);
impl<'a: 'b, 'b> Iterator for ArgsIter<'a, 'b> {
    type Item = Option<Datum<'a>>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let (&datum, &null) = self.0.next()?;
        Some(if null == 0 {
            Some(datum)
        } else {
            None
        })
    }
}
impl<'a: 'b, 'b> ExactSizeIterator for ArgsIter<'a, 'b> {
    fn len(&self) -> usize {
        self.0.len()
    }
}


#[macro_export]
macro_rules! lifetimeize {
    (bytea) => ( $crate::varlena::Toasted<'a, $crate::types::bytea> );
    (text) => ( $crate::varlena::Toasted<'a, $crate::types::text> );
    ($other:ident) => ( $crate::types::$other );
}


use std::cell::Cell;
use std::panic::AssertUnwindSafe;
pub struct FunctionCallContext<'a> {
    fcinfo: FunctionCallInfo<'a>,
    allocator: ManuallyDrop<MemoryContext<'static>>,
    //spi: Option<SpiContext>;
    has_spi: AssertUnwindSafe<Cell<bool>>,
}


impl<'a> FunctionCallContext<'a> {
    pub fn fcinfo(&self) -> &FunctionCallInfo<'a> {
        &self.fcinfo
    }

    pub fn allocator(&self) -> &MemoryContext<'static> {
        &self.allocator
    }

    pub fn alloc_bytea(&self, len: usize) -> &'a mut bytea {
        #[allow(mutable_transmutes)]
        unsafe {
            let size = len + 4;
            let ptr = self.allocator.alloc(size).as_mut_ptr() as *mut u32;
            *ptr = (size as u32) << 2;
            let shared: &'a bytea = <Toasted<bytea> as FromDatum>::from(Datum::create(ptr as usize)).to_varlena().unwrap();
            mem::transmute(shared)
        }
    }

    pub fn connect_spi(&self) -> SpiContext {
        assert!(!self.has_spi.get(), "Multiple SPI connections are illegal!");

        self.has_spi.set(true);

        unsafe { SpiContext::create() }
    }
}

// TODO: support strict functions properly (omit check entirely)
macro_rules! CREATE_STRICT_FUNCTION {
    ( fn $fname:ident @ $finfo:ident ( $context:ident , $( $argname:ident : $argty:ident ),* ) -> $retty:ident $body:block ) => {
        CREATE_FUNCTION! {
            fn $fname @ $finfo ( $context, $( $argname : $argty ),* ) -> $retty {
                $(
                    let $argname = $argname ?;
                )*;
                $body
            }
        }
    }
}

#[macro_export]
macro_rules! lowlevel_export {
    ( fn $fname:ident @ $finfo:ident ( $fcinfo:ident ) $body:block ) => {
        #[no_mangle]
        pub extern "C" fn $finfo () -> *const $crate::export::Pg_finfo_record {
            static FINFO: $crate::export::Pg_finfo_record = $crate::export::Pg_finfo_record { version: 1 };
            &FINFO
        }

        #[no_mangle]
        pub unsafe extern "C" fn $fname <'a>(fcinfo: $crate::export::FunctionCallInfo<'a>) -> Datum<'a> {
            fn user_impl<'a>($fcinfo: $crate::export::FunctionCallInfo<'a>) -> Datum<'a> {
                $body
            }

            error::convert_rust_panic(|| user_impl(fcinfo))
        }
    }
}

#[macro_export]
macro_rules! CREATE_FUNCTION {
    ( fn $fname:ident @ $finfo:ident ( $context:ident $( , $argname:ident : $argty:ident )* ) $body:block ) => {
        CREATE_FUNCTION! {
            fn $fname @ $finfo ( $context $( , $argname : $argty )* ) -> void { let () = $body; Some(()) }
        }
    };

    ( fn $fname:ident @ $finfo:ident ( $context:ident  $( , $argname:ident : $argty:ident )* ) -> $retty:ident $body:block ) => {
        // non-SRF case
        #[no_mangle]
        // this does not work: #[link_name = concat!("pg_finfo_", stringify!($fname))]
        pub extern "C" fn $finfo () -> *const $crate::export::Pg_finfo_record {
            static FINFO: $crate::export::Pg_finfo_record = $crate::export::Pg_finfo_record { version: 1 };
            &FINFO
        }

        #[no_mangle]
        #[allow(unused_mut)]
        pub unsafe extern "C" fn $fname (fcinfo: $crate::export::FunctionCallInfo) -> Datum {
            #[inline(always)]
            fn user_impl <'a> ( $context : &$crate::export::FunctionCallContext<'a>, $( $argname : Option< lifetimeize!($argty) > ),* ) -> Option< lifetimeize!($retty) > {
                $body
            }

            static KNOWN_GOOD_OID: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::ATOMIC_USIZE_INIT;
            static EXPECTED_ARG_TYPES: &'static [$crate::types::Oid] = &[
                $( <$crate::types::$argty as $crate::types::StaticallyTyped>::OID ),*
            ];

            let ctx = fcinfo.bootstrap();
            let fcinfo = ctx.fcinfo();
            let ret = $crate::error::convert_rust_panic(|| {
                let mut args = fcinfo.args();
                let mut arg_types = fcinfo.arg_types();

                // we don't support variadic shit
                assert_eq!(args.len(), arg_types.len());
                //assert_eq!(args.len(), EXPECTED_ARG_TYPES.len());

                // hopefully we can elide typechecking?
                let my_oid_usz = fcinfo.oid().0 as usize;
                if my_oid_usz != KNOWN_GOOD_OID.load(::std::sync::atomic::Ordering::Relaxed) {
                    // first call, have to typecheck
                    fcinfo.typecheck($crate::types::$retty::OID, EXPECTED_ARG_TYPES);
                    // remember that this is done
                    KNOWN_GOOD_OID.store(my_oid_usz, ::std::sync::atomic::Ordering::Relaxed);
                }

                // finally, read the actual parameters
                $(
                    let $argname = args.next().unwrap().map(|d| $crate::types::FromDatum::from(d)); // unwrap can't trigger, length is already checked
                )*;

                user_impl(&ctx,
                    $(
                        $argname
                    ),*
                )
            });
            match ret {
                Some(x) => Datum::from(x),
                None => fcinfo.return_null(),
            }
        }
    };
}
