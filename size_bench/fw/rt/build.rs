//! Puts `link.x` on the linker's search path, so an image links with `-C link-arg=-Tlink.x`.

use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::copy("link.x", out.join("link.x")).expect("copy link.x into OUT_DIR");
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=link.x");
}
