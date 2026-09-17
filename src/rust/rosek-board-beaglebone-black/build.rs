use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={manifest_dir}");
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    println!("cargo:rerun-if-changed=linker.ld");

    generate_config();
}

/// Parses rosek.cfg (issue #7 grammar) and writes the generated
/// TASKS/OS_APPLICATIONS Rust source to $OUT_DIR/config.rs, included by
/// main.rs. Issue #9.
fn generate_config() {
    let cfg_path = "rosek.cfg";
    println!("cargo:rerun-if-changed={cfg_path}");

    let source = std::fs::read_to_string(cfg_path)
        .unwrap_or_else(|e| panic!("failed to read {cfg_path}: {e}"));

    let config =
        rosek_config::parse(&source).unwrap_or_else(|e| panic!("failed to parse {cfg_path}: {e}"));

    let generated = rosek_config::codegen::generate(&config)
        .unwrap_or_else(|e| panic!("failed to generate code from {cfg_path}: {e}"));

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("config.rs");
    std::fs::write(&out_path, generated)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", out_path.display()));
}
