extern crate cc;

use std::process::Command;
use std::str;

fn main() {
    let output = Command::new("pg_config").arg("--includedir-server").output().unwrap().stdout;
    let includedir = str::from_utf8(&output).unwrap().trim();
    cc::Build::new()
        .file("src/gluedefs.c")
        .include(includedir)
        .compile("gluedefs");
}
