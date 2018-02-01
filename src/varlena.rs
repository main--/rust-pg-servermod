/*use std::marker::PhantomData;
use std::slice;

use alloc::MemoryContext;*/

// compare postgres.h for documentation on how this format works

// the great tragedy of this implementation is that extern types are still unstable
// so instead we use a [u8; 0] to somehow make this a dst
// this turns every reference into a fat ptr though which in turn requires more ugly hacks
pub struct BaseVarlena {
    _inner: [u8],
}

macro_rules! dst_ptrcast {
    ($ptr:expr) => (&mut *(::std::slice::from_raw_parts_mut($ptr as *const u8 as *mut u8, 0) as *mut _ as *mut _))
}


// TODO: implement toasting
/*



enum Header {
    External, // 1B_E aka TOAST
    Small(u8), // 1B
    Large(u32), // 4B_U
    LargeCompressed(u32), // 4B_C
}

extern "C" {
    fn pg_detoast_datum_copy(datum: *const u8) -> *mut u8;
}

impl<'a> Varlena<'a> {
    pub fn try_open(&self) -> Option<&[u8]> {
        unsafe {
            match self.header() {
                Header::Small(s) => Some(&slice::from_raw_parts(self.ptr, s as usize)[1..]),
                Header::Large(s) => Some(&slice::from_raw_parts(self.ptr, s as usize)[4..]),
                _ => None,
            }
        }
    }

    pub fn copy_detoast<'b, 'c>(&self, allocator: &'b MemoryContext<'c>) -> OwnedVarlena<'b> {
        unsafe {
            allocator.set_current();
            OwnedVarlena { ptr: pg_detoast_datum_copy(self.ptr), lifetime: PhantomData }
        }
    }

    pub fn detoast_packed<'b, 'c: 'b, 'd>(&self, allocator: &'c MemoryContext<'d>) -> VarlenaCow<'b> where 'a: 'b {
        unsafe {
            match self.header() {
                Header::Small(_) | Header::Large(_) => VarlenaCow::Borrowed(*self),
                _ => VarlenaCow::Owned(self.copy_detoast(allocator)),
            }
        }
    }
    
    #[cfg(target_endian = "little")]
    unsafe fn header(&self) -> Header {
        let first = *self.ptr;
        match first & 0x03 {
            0x00 => Header::Large((*(self.ptr as *const u32)) >> 2),
            0x02 => Header::LargeCompressed((*(self.ptr as *const u32)) >> 2),
            _ if first == 0x01 => Header::External,
            _ => Header::Small(first >> 1),
        }
    }

    #[cfg(target_endian = "big")]
    unsafe fn header(&self) -> Header {
        let first = *self.ptr;
        match first & 0xC0 {
            0x00 => Header::Large((*(self.ptr as *const u32)) >> 2),
            0x40 => Header::LargeCompressed((*(self.ptr as *const u32)) >> 2),
            _ if first == 0x80 => Header::External,
            _ => Header::Small(first & 0x7f),
        }
    }
}
*/
