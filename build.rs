use std::env;

use cmake;
use cxx_build;

fn main() {
    let mgclient = cmake::build("mgclient");

    cxx_build::bridge("src/clients/memgraph.rs")
        .file("src/clients/memgraph.cpp")
        .includes([
            "mgclient/mgclient_cpp/include",
            format!("{}/include", env::var("OUT_DIR").unwrap()).as_str(),
        ])
        .compile("memgraph-ops");

    // static libs comes last
    println!("cargo:rustc-link-search=native={}/lib", mgclient.display());
    println!("cargo:rustc-link-lib=static=mgclient");
}
