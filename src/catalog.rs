use std::os::raw::c_char;
use std::ffi::CStr;
use Datum;
use types::{name, Oid};

// TODO: pg10 has exported SearchSysCache1, but 9.5 does not

/*
extern HeapTuple SearchSysCache(int cacheId,
                                Datum key1, Datum key2, Datum key3, Datum key4);
*/

#[repr(i32)]
enum SysCacheId {
    Type = ::CACHEID_TYPEOID,
}

//type HeapTuple = *const c_void;

// TODO: verify in build that C compiler is capable of aligning
#[repr(C)]
struct ItemPointerData {
    foo: [u8; 6]
}

#[repr(C)]
struct HeapTupleHeader {
    pad: [u8; 22],
    t_hoff: u8,
    // tail ...
}

#[repr(C)]
struct HeapTuple {
    t_len: u32,
    t_self: ItemPointerData,
    t_tableOid: Oid,
    t_data: *const HeapTupleHeader,
}

extern {
    // TODO: figure out precise version boundaries here
    #[cfg(postgres = "10.0")]
    fn SearchSysCache1(cacheid: SysCacheId, key1: Datum) -> *const HeapTuple;
    #[cfg(postgres = "9.5")]
    fn SearchSysCache(cacheid: SysCacheId, key1: Datum, key2: Datum, key3: Datum, key4: Datum) -> *const HeapTuple;
    fn ReleaseSysCache(tuple: *const HeapTuple);
}

#[cfg(postgres = "9.5")]
unsafe fn SearchSysCache1(cacheid: SysCacheId, key1: Datum) -> *const HeapTuple {
    let z = Datum::from(0);
    SearchSysCache(cacheid, key1, z, z, z)
}

struct pg_type {
    typname: name,
    // ...
}

pub struct Type {
    ptr: *const HeapTuple,
}

impl Type {
    pub fn new(oid: Oid) -> Option<Type> {
        unsafe {
            let tup = SearchSysCache1(SysCacheId::Type, oid.into());
            if tup.is_null() {
                None
            } else {
                Some(Type { ptr: tup })
            }
        }
    }

    unsafe fn getstruct(&self) -> *const pg_type {
        let hdr = (*self.ptr).t_data;
        ((hdr as usize) + ((*hdr).t_hoff as usize)) as *const pg_type
    }

    pub fn name(&self) -> &CStr {
        unsafe {
            CStr::from_ptr((*self.getstruct()).typname.as_ptr())
        }
    }
}

// convenience
pub fn get_type_name(oid: Oid) -> Option<String> {
    let typ = Type::new(oid)?;
    Some(typ.name().to_string_lossy().into_owned())
}

impl Drop for Type {
    fn drop(&mut self) {
        unsafe {
            ReleaseSysCache(self.ptr);
        }
    }
}
