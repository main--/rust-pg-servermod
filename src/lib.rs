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
mod relation;
pub mod access;
pub mod interrupt;
pub mod tuple;
pub mod spi;

// macro-internal modules
#[doc(hidden)] pub mod magic;
#[doc(hidden)] pub mod error;



// TODO: repr(transparent) EVERYWHERE, esp. here
// TODO: define datum as nonzero
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Datum<'a>(usize, ::std::marker::PhantomData<&'a ()>);
impl<'a> Datum<'a> {
    pub fn create(value: usize) -> Datum<'a> {
        Datum(value, ::std::marker::PhantomData)
    }
}
use std::fmt::{Debug, Formatter, Result as FmtResult};
impl<'a> Debug for Datum<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "DATUM[{:p}]", self.0 as *mut ())
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
        let v = v?.detoast_packed(ctx.allocator());

        let mut newbuf = ctx.alloc_bytea(v.len() + 1);
        for (a, &b) in newbuf.iter_mut().zip(v.iter()) {
            *a = b;
        }
        *newbuf.last_mut().unwrap() = 42;
        Some(newbuf.into())
    }
}

CREATE_FUNCTION! {
    fn demo @ pg_finfo_demo (ctx, b: bytea, i: int4) -> int8 {
        let b = b?.detoast_packed(ctx.allocator());

        let sum: i64 = b.iter().map(|&x| x as i64).sum();
        Some(sum + (i? as i64) + 42)
    }
}

CREATE_FUNCTION! {
    fn rbitset_add @ pg_finfo_rbitset_add (ctx, b: bytea, i: int4) -> bytea {
        use std::cmp;

        let b = b?.detoast_packed(ctx.allocator());
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
    fn rbitand_count @ pg_finfo_rbitand_count (ctx, a: bytea, b: bytea) -> int4 {
        let a = a?.detoast_packed(ctx.allocator());
        let b = b?.detoast_packed(ctx.allocator());

        let sum: u32 = a.iter().cloned().zip(b.iter().cloned()).map(|(a, b)| (a & b).count_ones()).sum();
        Some(sum as i32)
    }
}

CREATE_FUNCTION! {
    fn errtest @ pg_finfo_errtest (_ctx) -> void {
        error::convert_postgres_error(|| error::convert_rust_panic(|| panic!("inney")))
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
    fn spass @ pg_finfo_spass (_ctx, a: text) -> text {
        Some(a)
    }
}

CREATE_STRICT_FUNCTION! {
    fn ptext @ pg_finfo_ptext (ctx, a: text, b: text) -> int4 {
//        let a = a.to_varlena()?;
        let a = a.copy_detoast(ctx.allocator());
        let b = b.to_varlena()?;
        let a: i32 = a.to_str()?.parse().ok()?;
        let b: i32 = b.to_str()?.parse().ok()?;
        Some(a + b)
    }
}

CREATE_STRICT_FUNCTION! {
    fn scanheap @ pg_finfo_scanheap(ctx, id: oid) -> int4 {
        let heap = access::heap::Heap::open(id);
        let mut scan = heap.scan(ctx.allocator());

        while let Some(x) = scan.next() {
            println!("{:?} {:?} {:?}", x.attribute(1), x.attribute(2), x.attribute(-7));
        }

        Some(42)
    }
}


CREATE_STRICT_FUNCTION! {
    fn scanindex @ pg_finfo_scanindex(ctx, heap: oid, index: oid, col: int4, val: int4) -> int4 {
        let heap = access::heap::Heap::open(heap);
        let index = access::index::Index::open(index);
        let keys = [access::index::ScanKey::new(col as u16, val as usize)];
        let mut scan = index.scan(&heap, &keys, ctx.allocator());

        while let Some(x) = scan.next() {
           println!("{:?}", x);
        }

        Some(42)
    }
}


CREATE_FUNCTION! {
    fn canceldota @ pg_finfo_canceldota ( _ctx ) -> void {
        loop {
            interrupt::check_for_interrupts();
            println!("lul");
        }
    }
}

CREATE_STRICT_FUNCTION! {
    fn evalul @ pg_finfo_evalul(ctx, sql: text) -> int4 {
        let sql = sql.detoast_packed(ctx.allocator()).to_str()?;

        let spi = ctx.connect_spi();
        let res = spi.execute(sql, &[42i32.into(), ::spi::Parameter::null::<i32>(), 1337i32.into()]).unwrap();
        println!("{:?}", res);

        Some(42)
    }
}

CREATE_STRICT_FUNCTION! {
    fn evalul2 @ pg_finfo_evalul2(ctx, sql: text) -> int4 {
        let sql = sql.detoast_packed(ctx.allocator()).to_str()?;

        let spi = ctx.connect_spi();
        let mut cursor = spi.execute_cursor(sql, &[42i32.into(), ::spi::Parameter::null::<i32>(), 1337i32.into()]);
        println!("{:?}", cursor.fetch(::spi::Direction::Forward, 2));
        cursor.move_relative(1);
        println!("{:?}", cursor.fetch(::spi::Direction::Forward, 1));
        cursor.move_relative(-1);
        println!("{:?}", cursor.fetch(::spi::Direction::Backward, 1));
        cursor.move_absolute(5);
        println!("{:?}", cursor.fetch(::spi::Direction::Forward, 1));

        Some(42)
    }
}
