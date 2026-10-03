fn main() {
    let crate_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let config =
        cbindgen::Config::from_file("cbindgen.toml")
            .expect("cbindgen.toml not found");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .with_src("src/main.rs")
        .generate()
        .expect("cbindgen failed to generate EthTrcv.h")
        .write_to_file("include/EthTrcv.h");
}
