// contract: everything in here is StaticallyTyped + FromDatum + Into<Datum>
use std::os::raw::{c_void, c_char};

use super::Datum;
use varlena::BaseVarlena;
use alloc::MemoryContext;
use error;

mod hack { pub type bool_hack = bool; }
pub type bool = self::hack::bool_hack;
pub type char = i8;
pub type int8 = i64;
pub type int2 = i16;
pub type int4 = i32;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Oid(pub u32);
// TODO: rename Oid to oid
pub type oid = Oid;

// TODO: proper toast api in varlena.rs instead of this hack
// FIXME: current impl is even wrong - this invocation relies on the current memory context
// (we don't take one at all)
extern "C" {
    fn pg_detoast_datum_packed(p: *mut c_void) -> *mut c_void;
}


impl DerefMut for bytea {
    fn deref_mut(&mut self) -> &mut [u8] {
        let ptr = self as *mut _ as *mut u8;
        unsafe {
            let head8 = *ptr;
            if (head8 & 0x01) != 0 {
                panic!("Attempted to write to a bytea with 1-byte header.");
            }

            let len = *(ptr as *const u32) >> 2; // total size
            &mut slice::from_raw_parts_mut(ptr, len as usize)[4..] // skip header
        }
    }
}


use std::ops::{Deref, DerefMut};
use std::slice;
impl Deref for bytea {
    type Target = [u8];
    
    fn deref(&self) -> &[u8] {
        let ptr = self as *const _ as *mut u8;
        unsafe {
            // FIXME: we assume little endian
            let head8 = *ptr;
            if (head8 & 0x01) != 0 {
                // is 1b
                let len = head8 >> 1; // total size
                &slice::from_raw_parts(ptr, len as usize)[1..] // skip header
            } else {
                // is 4b
                let len = *(ptr as *const u32) >> 2; // total size
                &slice::from_raw_parts(ptr, len as usize)[4..] // skip header
            }
        }
    }
}

pub struct bytea(BaseVarlena);
pub struct text(BaseVarlena);

#[repr(i32)]
enum PgEncoding {
    SqlAscii = 0,
    Utf8 = 6,
    // rest is todo
    // fixme: generate these from pgbuild
}

extern "C" {
    fn GetDatabaseEncoding() -> i32;
    fn pg_server_to_any(s: *const c_char, len: i32, encoding: i32) -> *const c_char;
}



use std::str;
use std::ffi::CStr;
impl text {
    pub fn to_string<'a, 'b>(&'a self, alloc: &'b MemoryContext) -> Result<&'a str, &'b mut str> {
        unsafe {
            let my_data: &bytea = ::std::mem::transmute(self);
            let myptr = my_data.as_ptr() as *const c_char;
            alloc.set_current();
            let converted = error::convert_postgres_error(|| pg_server_to_any(myptr, my_data.len() as i32, PgEncoding::Utf8 as i32));
            if converted == myptr {
                Ok(str::from_utf8_unchecked(my_data))
            } else {
                // poor man's strlen (can't do this naturally through CStr - would have to mut-transmute the &str)
                let newlen = CStr::from_ptr(converted).to_bytes().len();
                Err(str::from_utf8_unchecked_mut(slice::from_raw_parts_mut(converted as *mut u8, newlen)))
            }
        }
    }

    // edgecase: for invalid utf, to_string should error while this just returns None
    // is that a problem?
    pub fn to_str(&self) -> Option<&str> {
        unsafe {
            let my_data: &bytea = ::std::mem::transmute(self);
            let encoding = GetDatabaseEncoding();
            if encoding == PgEncoding::Utf8 as i32 {
                Some(str::from_utf8_unchecked(my_data))
            } else if encoding == PgEncoding::SqlAscii as i32 {
                str::from_utf8(my_data).ok()
            } else {
                None
            }
        }
    }
}

pub type name = [c_char; 64]; // FIXME wtf

pub type void = ();

impl<'a> From<&'a text> for Datum<'a> { fn from(b: &'a text) -> Datum<'a> { Datum::create(b as *const _ as *const c_void as usize) } }
impl<'a> FromDatum<'a> for &'a text { unsafe fn from(d: Datum<'a>) -> &'a text { dst_ptrcast!(pg_detoast_datum_packed(d.0 as *mut _)) } }
impl<'a> From<&'a bytea> for Datum<'a> { fn from(b: &'a bytea) -> Datum<'a> { Datum::create(b as *const _ as *const c_void as usize) } }
impl<'a> FromDatum<'a> for &'a bytea { unsafe fn from(d: Datum<'a>) -> &'a bytea { dst_ptrcast!(pg_detoast_datum_packed(d.0 as *mut _)) } }

impl<'a> From<oid> for Datum<'a> { fn from(i: oid) -> Datum<'a> { Datum::create(i.0 as usize) } }
impl<'a> FromDatum<'a> for oid { unsafe fn from(d: Datum<'a>) -> oid { Oid(d.0 as u32) } }
impl<'a> From<int8> for Datum<'a> { fn from(i: i64) -> Datum<'a> { Datum::create(i as usize) } }
impl<'a> FromDatum<'a> for int8 { unsafe fn from(d: Datum<'a>) -> i64 { d.0 as i64 } }
impl<'a> From<int4> for Datum<'a> { fn from(i: i32) -> Datum<'a> { Datum::create(i as usize) } }
impl<'a> FromDatum<'a> for int4 { unsafe fn from(d: Datum<'a>) -> i32 { d.0 as i32 } }

// can't use regular(safe) From:
// e.g. datum -> bytea imples that datum is a valid ptr

pub trait FromDatum<'a> {
    unsafe fn from(datum: Datum<'a>) -> Self;
}


pub unsafe trait StaticallyTyped { const OID: Oid; }
unsafe impl StaticallyTyped for bool { const OID: Oid = Oid(16); }
unsafe impl StaticallyTyped for bytea { const OID: Oid = Oid(17); }
unsafe impl StaticallyTyped for char { const OID: Oid = Oid(18); }
unsafe impl StaticallyTyped for name { const OID: Oid = Oid(19); }
unsafe impl StaticallyTyped for int8 { const OID: Oid = Oid(20); }
unsafe impl StaticallyTyped for int2 { const OID: Oid = Oid(21); }
unsafe impl StaticallyTyped for int4 { const OID: Oid = Oid(23); }
unsafe impl StaticallyTyped for text { const OID: Oid = Oid(25); }
unsafe impl StaticallyTyped for Oid { const OID: Oid = Oid(26); }

// void type:
impl<'a> From<void> for Datum<'a> { fn from(_: ()) -> Datum<'a> { Datum::create(0) } }
impl<'a> FromDatum<'a> for void { unsafe fn from(_: Datum<'a>) { } }
unsafe impl StaticallyTyped for void { const OID: Oid = Oid(2278); }





/*
#define INT2VECTOROID   22
#define REGPROCOID              24
#define TIDOID          27
#define XIDOID 28
#define CIDOID 29
#define OIDVECTOROID    30
#define JSONOID 114
#define XMLOID 142
#define PGNODETREEOID   194
#define PGDDLCOMMANDOID 32
#define POINTOID                600
#define LSEGOID                 601
#define PATHOID                 602
#define BOXOID                  603
#define POLYGONOID              604
#define LINEOID                 628
#define FLOAT4OID 700
#define FLOAT8OID 701
#define ABSTIMEOID              702
#define RELTIMEOID              703
#define TINTERVALOID    704
#define UNKNOWNOID              705
#define CIRCLEOID               718
#define CASHOID 790
#define MACADDROID 829
#define INETOID 869
#define CIDROID 650
#define INT2ARRAYOID            1005
#define INT4ARRAYOID            1007
#define TEXTARRAYOID            1009
#define OIDARRAYOID                     1028  
#define FLOAT4ARRAYOID 1021
#define ACLITEMOID              1033
#define CSTRINGARRAYOID         1263
#define BPCHAROID               1042
#define VARCHAROID              1043
#define DATEOID                 1082
#define TIMEOID                 1083
#define TIMESTAMPOID    1114
#define TIMESTAMPTZOID  1184
#define INTERVALOID             1186
#define TIMETZOID               1266
#define BITOID   1560
#define VARBITOID         1562
#define NUMERICOID              1700
*/
