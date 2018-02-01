#![allow(non_snake_case, non_camel_case_types)]


use std::mem;

use types::StaticallyTyped;


include!(concat!(env!("OUT_DIR"), "/basedefs.rs"));

// external modules
pub mod alloc;
#[macro_use] pub mod varlena;
pub mod types;
#[macro_use] pub mod export;
pub mod catalog;
pub mod index;

// macro-internal modules
#[doc(hidden)] pub mod magic;
#[doc(hidden)] pub mod error;



// TODO: repr(transparent) EVERYWHERE, esp. here
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Datum<'a>(usize, ::std::marker::PhantomData<&'a ()>);
impl<'a> Datum<'a> {
    pub fn create(value: usize) -> Datum<'a> {
        Datum(value, ::std::marker::PhantomData)
    }
}



// TODO: SRF (with generators)


lowlevel_export! {
    fn lowlevel @ pg_finfo_lowlevel(_fcinfo) {
        Datum::create(0)
    }
}

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
        Some(newbuf)
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
        Some(newbuf)
    }
}

CREATE_FUNCTION! {
    fn rbitand_count @ pg_finfo_rbitand_count (_ctx, a: bytea, b: bytea) -> int4 {
        let sum: u32 = a?.iter().cloned().zip(b?.iter().cloned()).map(|(a, b)| (a & b).count_ones()).sum();
        Some(sum as i32)
    }
}

CREATE_FUNCTION! {
    fn errtest @ pg_finfo_errtest (_ctx) -> void {
        unsafe {
            error::convert_postgres_error(|| error::convert_rust_panic(|| panic!("inney")))
        }
    }
}

/*
CREATE_STRICT_FUNCTION! {
    fn scantest @ pg_finfo_scantest (_ctx, rel: Oid, idx: Oid) -> int4 {
        Some(index::do_index_scan(rel, idx))
    }
}
*/

CREATE_STRICT_FUNCTION! {
    fn typname @ pg_finfo_typname (_ctx, typ: Oid) -> int4 {
        let cat = catalog::Type::new(typ).unwrap();
        panic!("type {} is called {:?}", typ.0, cat.name());
    }
}


CREATE_STRICT_FUNCTION! {
    fn ptext @ pg_finfo_ptext (_ctx, a: text, b: text) -> int4 {
        let a: i32 = a.to_str()?.parse().ok()?;
        let b: i32 = b.to_str()?.parse().ok()?;
        Some(a + b)
    }
}
