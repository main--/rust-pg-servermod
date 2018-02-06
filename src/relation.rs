use std::os::raw::c_void;
use types::Oid;

#[repr(C, packed)]
pub struct Relation {
    _pad: [u8; ::RELATT_OFFSET],
    pub td: *const RawTupleDesc,
}

#[repr(C, packed)]
pub struct RawTupleDesc {
    pub natts: i32,
    pub tdtypeid: Oid,
    // ...
}

extern "C" {
    // TODO: proper transaction api
    pub fn GetTransactionSnapshot() -> *mut c_void;
}
