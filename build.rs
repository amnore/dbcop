use std::env;

use cmake;
use cxx_build;

fn main() {
    let mgclient = cmake::build("mgclient");

    cxx_build::bridge("src/clients/memgraph.rs")
        .file("src/clients/memgraph.cpp")
        .flag("-std=c++17")
        .includes([
            "mgclient/mgclient_cpp/include",
            format!("{}/include", env::var("OUT_DIR").unwrap()).as_str(),
        ])
        .compile("memgraph-ops");

    // static libs comes last
    for path in [ "lib", "lib64", "lib32" ] {
        println!("cargo:rustc-link-search=native={}/{}", mgclient.display(), path);
    }
    println!("cargo:rustc-link-lib=static=mgclient");
}
