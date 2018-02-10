use std::marker::PhantomData;

use Datum;
use types::FromDatum;
use alloc::MemoryContext;

// compare postgres.h for documentation on how this format works

// the great tragedy of this implementation is that extern types are still unstable
// so instead we use a [u8; 0] to somehow make this a dst
// this turns every reference into a fat ptr though which in turn requires more ugly hacks
pub struct BaseVarlena {
    _inner: [u8],
}

pub unsafe trait Varlena {
    unsafe fn dst_ptrcast<'a, P>(ptr: *const P) -> &'a mut Self;
}


macro_rules! impl_varlena {
    ($name:ident) => {
        unsafe impl $crate::varlena::Varlena for $name {
            unsafe fn dst_ptrcast<'a, P>(ptr: *const P) -> &'a mut Self {
                &mut *(::std::slice::from_raw_parts_mut(ptr as *const u8 as *mut u8, 0) as *mut _ as *mut _)
            }
        }
    }
}

enum Header {
    External, // 1B_E aka TOAST
    Small(u8), // 1B
    Large(u32), // 4B_U
    LargeCompressed(u32), // 4B_C
}

#[repr(C)]
pub struct Toasted<'a, T: 'a + Varlena + ?Sized> {
    ptr: *const u8,
    marker: PhantomData<&'a T>,
}

impl<'a, T: 'a + Varlena + ?Sized> FromDatum<'a> for Toasted<'a, T> {
    unsafe fn from(d: Datum<'a>) -> Toasted<'a, T> {
        Toasted {
            ptr: d.0 as *const u8,
            marker: PhantomData,
        }
    }
}
impl<'a, T: 'a + Varlena + ?Sized> From<Toasted<'a, T>> for Datum<'a> {
    fn from(b: Toasted<'a, T>) -> Datum<'a> {
        Datum::create(b.ptr as usize)
    }
}
impl<'a, T: 'a + Varlena + ?Sized> From<&'a T> for Toasted<'a, T> {
    fn from(r: &'a T) -> Toasted<'a, T> {
        Toasted {
            ptr: r as *const _ as *const u8,
            marker: PhantomData,
        }
    }
}
impl<'a, T: 'a + Varlena + ?Sized> From<&'a mut T> for Toasted<'a, T> {
    fn from(r: &'a mut T) -> Toasted<'a, T> {
        Toasted {
            ptr: r as *const _ as *const u8,
            marker: PhantomData,
        }
    }
}

extern "C" {
    fn pg_detoast_datum_copy(datum: *const u8) -> *mut u8;
}

impl<'a, T: 'a + Varlena + ?Sized> Toasted<'a, T> {
    pub fn to_varlena(&self) -> Option<&'a T> {
        unsafe {
            match self.header() {
                Header::Small(_) | Header::Large(_) => {
                    let r: &'a T = T::dst_ptrcast(self.ptr);
                    Some(r)
                }
                _ => None,
            }
        }
    }

    pub fn copy_detoast<'b, 'c>(&self, allocator: &'b MemoryContext<'c>) -> &'b mut T {
        unsafe {
            allocator.set_current();
            T::dst_ptrcast(pg_detoast_datum_copy(self.ptr))
        }
    }

    pub fn detoast_packed<'b, 'c, 'd>(&self, allocator: &'c MemoryContext<'d>) -> &'b T where 'c: 'b, 'a: 'b {
        self.to_varlena().unwrap_or_else(|| self.copy_detoast(allocator))
    }

    unsafe fn header(&self) -> Header {
        header(self.ptr)
    }
}

#[cfg(target_endian = "little")]
unsafe fn header(ptr: *const u8) -> Header {
    let first = *ptr;
    match first & 0x03 {
        0x00 => Header::Large((*(ptr as *const u32)) >> 2),
        0x02 => Header::LargeCompressed((*(ptr as *const u32)) >> 2),
        _ if first == 0x01 => Header::External,
        _ => Header::Small(first >> 1),
    }
}

#[cfg(target_endian = "big")]
unsafe fn header(ptr: *const u8) -> Header {
    let first = *ptr;
    match first & 0xC0 {
        0x00 => Header::Large((*(ptr as *const u32)) & 0x3FFFFFFF),
        0x40 => Header::LargeCompressed((*(ptr as *const u32)) & 0x3FFFFFFF),
        _ if first == 0x80 => Header::External,
        _ => Header::Small(first & 0x7f),
    }
}
