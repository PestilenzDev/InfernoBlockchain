// build.rs
use std::path::Path;

fn main() {
    let libtorch_path = Path::new("libtorch/lib");
    println!("cargo:rustc-link-search=native={}", libtorch_path.display());
    println!("cargo:rustc-link-lib=dylib=torch_cpu");
    println!("cargo:rustc-link-lib=dylib=torch");
    println!("cargo:rustc-link-lib=dylib=c10");
    println!("cargo:rerun-if-changed=libtorch");
}