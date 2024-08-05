extern crate cbindgen;

use std::env;
use version_number::FullVersion;

fn main() {
    // VERSION

    let v = FullVersion::parse(env!("CARGO_PKG_VERSION")).expect("Unexpected error.");
    let major = v.major as u32;
    let minor = v.minor as u32;
    let patch = v.patch as u32;

    let ver: u32 = patch | minor << 8 | major << 16;

    println!("cargo:rustc-env=XSYNTHVERSION={}", ver);

    // CBINDGEN

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut config = cbindgen::Config::from_file("cbindgen.toml")
        .expect("Unable to find cbindgen.toml configuration file");
    config.after_includes = Some(format!("\n#define XSYNTH_VERSION {:#x}", ver));

    cbindgen::Builder::new()
        .with_config(config)
        .with_crate(crate_dir)
        .with_parse_deps(true)
        .with_parse_include(&["xsynth-core", "xsynth-realtime"])
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("xsynth.h");
}
