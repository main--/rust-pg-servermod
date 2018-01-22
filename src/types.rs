// contract: everything in here is StaticallyTyped + FromDatum + Into<Datum>
use super::Datum;

mod hack { pub type bool_hack = bool; }
pub type bool = self::hack::bool_hack; // super::primitives_hack::bool;
pub type char = i8;
pub type int8 = i64;
pub type int2 = i16;
pub type int4 = i32;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Oid(pub u32);
// TODO: rename Oid to oid
pub type oid = Oid;

// TODO: some support to get type names. how you ask?
// simple:
// HeapTuple t = SearchSysCache1(TYPEOID, ObjectIdGetDatum(oid));
// if (!IsHeapTupleValid(t)) ...
// Form_pg_type typdesc = (Form_pg_type) GETSTRUCT(t);
// typedesc->typname
// voila your cstring name right there



// on varlena:
// all variable-size data structures in postgres (even custom ones) must use the varlena format
// it consists of either a 1-byte or a 4-byte header followed by the payload
// if the first header bit is unset, it's a 1-byte header wherein the remaining bits specify the varlena's length
// else it's a 4-byte which has an additional flag bit for TOAST which we ignore (not implemented) followed by again, the length
// NB length is always the entire length including header
//
// Because we can build any variable-length type from a &[u8], we use bytea as base type.
// This does not reflect postgresql's actual design but is merely a shortcut in our implementation.


#[derive(Clone, Copy)] // fixme not private
pub struct bytea_mut<'a>(pub *mut super::c_void, pub ::std::marker::PhantomData<&'a mut [u8]>);
impl<'a> From<bytea_mut<'a>> for bytea<'a> { fn from(b: bytea_mut<'a>) -> bytea<'a> { bytea(b.0, ::std::marker::PhantomData) } }
impl<'a> Deref for bytea_mut<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        let ptr = self.0 as *mut u8;
        unsafe {
            let head8 = *ptr;
            if (head8 & 0x01) != 0 {
                panic!("Attempted to read from a bytea_mut with 1-byte header.");
            }

            let len = *(ptr as *const u32) >> 2; // total size
            &mut slice::from_raw_parts_mut(ptr, len as usize)[4..] // skip header
        }
    }
}
impl<'a> DerefMut for bytea_mut<'a> {
    fn deref_mut(&mut self) -> &mut [u8] {
        let ptr = self.0 as *mut u8;
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


// TODO all byref-types need lifetimes (not just this one)
#[derive(Clone, Copy)]
pub struct bytea<'a>(*mut super::c_void, ::std::marker::PhantomData<&'a [u8]>);

use std::ops::{Deref, DerefMut};
use std::slice;
impl<'a> Deref for bytea<'a> {
    type Target = [u8];
    
    fn deref(&self) -> &[u8] {
        let ptr = self.0 as *mut u8;
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

pub type name = [i8; 64]; // FIXME wtf
pub struct text<'a>(bytea<'a>);

pub type void = ();

impl<'a> From<text<'a>> for Datum<'a> { fn from(t: text<'a>) -> Datum<'a> { t.0.into() } }
impl<'a> FromDatum<'a> for text<'a> { unsafe fn from(d: Datum<'a>) -> text<'a> { text(FromDatum::from(d)) } }
impl<'a> From<bytea<'a>> for Datum<'a> { fn from(b: bytea<'a>) -> Datum<'a> { Datum::create(b. 0 as usize) } }
impl<'a> FromDatum<'a> for bytea<'a> { unsafe fn from(d: Datum<'a>) -> bytea<'a> { bytea(super::pg_detoast_datum_packed(d.0 as *mut _), ::std::marker::PhantomData) } }

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
unsafe impl<'a> StaticallyTyped for bytea<'a> { const OID: Oid = Oid(17); }
unsafe impl StaticallyTyped for char { const OID: Oid = Oid(18); }
unsafe impl StaticallyTyped for name { const OID: Oid = Oid(19); }
unsafe impl StaticallyTyped for int8 { const OID: Oid = Oid(20); }
unsafe impl StaticallyTyped for int2 { const OID: Oid = Oid(21); }
unsafe impl StaticallyTyped for int4 { const OID: Oid = Oid(23); }
unsafe impl<'a> StaticallyTyped for text<'a> { const OID: Oid = Oid(25); }
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
