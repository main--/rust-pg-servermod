use std::os::raw::c_void;

use tupledesc::RawTupleDesc;

#[repr(C, packed)]
pub struct Relation {
    _pad: [u8; ::RELATT_OFFSET],
    pub td: *const RawTupleDesc,
}


extern "C" {
    // TODO: proper transaction api
    pub fn GetTransactionSnapshot() -> *mut c_void;
}
