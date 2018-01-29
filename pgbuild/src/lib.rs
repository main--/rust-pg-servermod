extern "C" {
    pub fn pg_version() -> u32;
    pub fn func_max_args() -> u32;
    pub fn index_max_keys() -> u32;
    pub fn namedatalen() -> u32;
    pub fn float4_byval() -> u32;
    pub fn float8_byval() -> u32;
    pub fn len_scankeydata() -> u32;
    pub fn len_sigjmpbuf() -> u32;

    pub fn cacheid_typeoid() -> u32;
}
