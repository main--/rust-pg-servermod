#![allow(non_snake_case, non_camel_case_types)]


use std::os::raw::c_void;
use std::{mem, ptr};

use types::{Oid, StaticallyTyped};


include!(concat!(env!("OUT_DIR"), "/basedefs.rs"));

// external modules
pub mod types;
#[macro_use] pub mod export;

// macro-internal modules
#[doc(hidden)] pub mod magic;
#[doc(hidden)] pub mod error;




#[repr(C)]
#[derive(Copy, Clone)]
pub struct Datum<'a>(usize, ::std::marker::PhantomData<&'a ()>);
impl<'a> Datum<'a> {
    pub fn create(value: usize) -> Datum<'a> {
        Datum(value, ::std::marker::PhantomData)
    }
}


// TODO: allocation api
// memory allocators doing their thing
// lifetimes saving the day
extern "C" {
    fn pg_detoast_datum_packed(p: *mut c_void) -> *mut c_void;
    fn palloc0(size: usize) -> *mut c_void;
}


type Relation = *mut c_void;
type IndexScanDesc = *mut c_void;
extern "C" {
    fn heap_open(relation: Oid, lockmode: i32) -> Relation;
    fn relation_close(relation: Relation, lockmode: i32);
    fn index_open(relation: Oid, lockmode: i32 /* set to 1 */) -> Relation; // open index relation
    fn index_close(relation: Relation, lockmode: i32);
    fn index_beginscan(heap: Relation, index: Relation, snapshot: *mut c_void /* null */, nkeys: i32, norderbys: i32) -> IndexScanDesc;
    fn index_rescan(scan: IndexScanDesc, scankeys: *mut u8, nkeys: i32, orderbys: *mut u8, norderbys: i32);
    fn index_getnext(scan: IndexScanDesc, direction: i32) -> *mut c_void; // returns HeapTuple
    fn index_getnext_tid(scan: IndexScanDesc, direction: i32) -> *mut c_void; // returns ItemPointer
    fn index_endscan(scan: IndexScanDesc);
    fn GetTransactionSnapshot() -> *mut c_void;
    fn ScanKeyInit(entry: *mut u8, attr_num: u16, strat_num: u16, regproc: u32, arg: usize);
}

// TODO: rework panic design!
// abolish pgpanic - it doesn't call rust dtors!
// instead always invoke the rust unwinder and catch at the native boundary
// slower but nothing we can do

fn do_index_scan(rel: Oid, idx: Oid) -> i32 {
    let mut counter = 0;
    unsafe {
        let heap = heap_open(rel, 1);
        let index = index_open(idx, 1);

        let btint4cmp = 184;
        let mut keybuf = [0u8; LEN_SCANKEYDATA];
        ScanKeyInit(keybuf.as_mut_ptr(), 1, 3, btint4cmp, 4);

        let snap = GetTransactionSnapshot();
        assert!(!snap.is_null());
        let scan = index_beginscan(heap, index, snap, 1, 0);
        index_rescan(scan, keybuf.as_mut_ptr(), 1, ptr::null_mut(), 0);
        loop {
            let thing = index_getnext(scan, 1);
            if thing.is_null() { break; }
            counter += 1;
        }
        index_endscan(scan);

        index_close(index, 1);
        relation_close(heap, 1);
    }
    counter
}


// TODO: SRF (with generators)


lowlevel_export! {
    fn lowlevel @ pg_finfo_lowlevel(_fcinfo) {
        Datum::create(0)
    }
}





//macro_rules! 


CREATE_FUNCTION! {
    fn bitadd_count @ pg_finfo_bitadd_count (_ctx, x: int4, y: int4) -> int4 {
        //unimplemented!();
        Some(x? + y?)
    }
}

CREATE_FUNCTION! {
    fn voidfun @ pg_finfo_voidfun (ctx, v: bytea) -> bytea {
        let mut newbuf = ctx.alloc_bytea(v?.len() + 1);
        for (a, &b) in newbuf.iter_mut().zip(v?.iter()) {
            *a = b;
        }
        *newbuf.last_mut().unwrap() = 42;
        Some(newbuf.into())
        //pgpanic!("val = {}", v?[1]);
            //None
    }
}

CREATE_FUNCTION! {
    fn demo @ pg_finfo_demo (_ctx, b: bytea, i: int4) -> int8 {
        let sum: i64 = b?.iter().map(|&x| x as i64).sum();
        Some(sum + (i? as i64) + 42)
    }
}

CREATE_FUNCTION! {
    fn rbitset_add @ pg_finfo_rbitset_add (ctx, b: bytea, i: int4) -> bytea {
        use std::cmp;
        let b = b?;
        let i = i?;
        if i < 0 { return None; }

        let byte_index = (i / 8) as usize;
        let required_size = byte_index + 1;
        
        let mut newbuf = ctx.alloc_bytea(cmp::max(b.len(), required_size));
        newbuf[..b.len()].copy_from_slice(&b);
        newbuf[byte_index] |= 1 << (i % 8);
        Some(newbuf.into())
    }
}

CREATE_FUNCTION! {
    fn rbitand_count @ pg_finfo_rbitand_count (_ctx, a: bytea, b: bytea) -> int4 {
        let sum: u32 = a?.iter().cloned().zip(b?.iter().cloned()).map(|(a, b)| (a & b).count_ones()).sum();
        Some(sum as i32)
    }
}

CREATE_STRICT_FUNCTION! {
    fn scantest @ pg_finfo_scantest (_ctx, rel: Oid, idx: Oid) -> int4 {
        Some(do_index_scan(rel, idx))
    }
}
