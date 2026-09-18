//! Process-level smoke tests for the `rosek-config-cli` binary. Uses
//! `CARGO_BIN_EXE_rosek-config-cli` (set automatically by Cargo for
//! integration tests of a crate with a same-named `[[bin]]`) instead of
//! pulling in a test-runner dependency.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rosek-config-cli"))
}

fn write_cfg(name: &str, contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rosek-config-cli-test-{}-{}",
        std::process::id(),
        name
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("rosek.cfg");
    std::fs::write(&path, contents).unwrap();
    path
}

const VALID_CFG: &str = "OS_APPLICATION App { TRUSTED = TRUE; };\n\
     TASK T { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = App; };\n";

#[test]
fn validate_succeeds_on_a_well_formed_config() {
    let path = write_cfg("validate-ok", VALID_CFG);
    let output = bin().arg("validate").arg(&path).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("OK (1 task(s), 1 os_application(s))"),
        "{stdout}"
    );
}

#[test]
fn validate_fails_on_an_unknown_application_reference() {
    let path = write_cfg(
        "validate-bad-ref",
        "TASK T { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = Ghost; };\n",
    );
    let output = bin().arg("validate").arg(&path).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Ghost"), "{stderr}");
}

#[test]
fn generate_prints_rust_source_to_stdout() {
    let path = write_cfg("generate-stdout", VALID_CFG);
    let output = bin().arg("generate").arg(&path).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub static TASKS"));
    assert!(stdout.contains("\"T\""));
}

#[test]
fn generate_writes_to_the_given_output_file() {
    let path = write_cfg("generate-file", VALID_CFG);
    let out_path = path.with_file_name("config.rs");
    let output = bin()
        .arg("generate")
        .arg(&path)
        .arg(&out_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let generated = std::fs::read_to_string(&out_path).unwrap();
    assert!(generated.contains("pub static OS_APPLICATIONS"));
}

#[test]
fn no_arguments_prints_usage_and_fails() {
    let output = bin().output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("usage:"), "{stderr}");
}
