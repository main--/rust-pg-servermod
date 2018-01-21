extern crate pgbuild;

use std::fs::File;
use std::path::Path;
use std::env;
use std::io::Write;

fn main() {
    // pgbuild functions are unsafe for no reason
    // but writing wrappers for every single one of them is rather pointless
    unsafe {
        println!("cargo:rustc-cfg=v{}", pgbuild::pg_version());

        let mut f = File::create(Path::new(&env::var("OUT_DIR").unwrap()).join("basedefs.rs")).unwrap();
        writeln!(f, "const PG_VERSION: usize = {};", pgbuild::pg_version()).unwrap();
        writeln!(f, "const FUNC_MAX_ARGS: usize = {};", pgbuild::func_max_args()).unwrap();
        writeln!(f, "const INDEX_MAX_KEYS: usize = {};", pgbuild::index_max_keys()).unwrap();
        writeln!(f, "const NAMEDATALEN: usize = {};", pgbuild::namedatalen()).unwrap();
        writeln!(f, "const FLOAT4_BYVAL: bool = {};", pgbuild::float4_byval() != 0).unwrap();
        writeln!(f, "const FLOAT8_BYVAL: bool = {};", pgbuild::float8_byval() != 0).unwrap();
        writeln!(f, "const LEN_SCANKEYDATA: usize = {};", pgbuild::len_scankeydata()).unwrap();
        writeln!(f, "const LEN_SIGJMPBUF: usize = {};", pgbuild::len_sigjmpbuf()).unwrap();
    }
}
