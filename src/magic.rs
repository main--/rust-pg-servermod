use super::*;

#[repr(C)]
pub struct Pg_magic_struct {
    len: i32,
    version: i32,
    funcmaxargs: i32,
    indexmaxkeys: i32,
    namedatalen: i32,
    float4byval: i32,
    float8byval: i32,
}

#[no_mangle]
#[doc(hidden)]
pub extern "C" fn Pg_magic_func() -> *const Pg_magic_struct {
    static MAGIC: Pg_magic_struct = Pg_magic_struct {
        len: mem::size_of::<Pg_magic_struct>() as i32,
        version: PG_VERSION as i32,
        funcmaxargs: FUNC_MAX_ARGS as i32,
        indexmaxkeys: INDEX_MAX_KEYS as i32,
        namedatalen: NAMEDATALEN as i32,
        float4byval: FLOAT4_BYVAL as i32,
        float8byval: FLOAT8_BYVAL as i32,
    };

    &MAGIC
}
