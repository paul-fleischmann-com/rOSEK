//! Scriptable generate/validate CLI for rOSEK's `.cfg` static
//! configuration files (see `rosek-config`'s grammar doc-comment and
//! `docs/arc42/08-concepts.adoc`). No TUI dependencies - meant for CI and
//! scripting; see `rosek-config-tui` for the interactive editor. Dev
//! tooling only, not part of the kernel, see `src/rust/Cargo.toml` for
//! why this crate lives outside the main workspace.
//!
//! ```text
//! rosek-config-cli generate <path.cfg>            print generated Rust to stdout
//! rosek-config-cli generate <path.cfg> <out.rs>    write generated Rust to a file
//! rosek-config-cli validate <path.cfg>             parse + validate, print OK or the error
//! ```

use rosek_config::{codegen, parse, Config};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("generate") => generate(&args[1..]),
        Some("validate") => validate(&args[1..]),
        _ => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!(
        "usage:\n  \
         rosek-config-cli generate <path.cfg> [out.rs]\n  \
         rosek-config-cli validate <path.cfg>"
    );
}

fn read_config(path: &str) -> Result<Config, String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    parse(&source).map_err(|e| format!("{path}: {e}"))
}

fn generate(args: &[String]) -> ExitCode {
    let Some(cfg_path) = args.first() else {
        print_usage();
        return ExitCode::FAILURE;
    };
    let config = match read_config(cfg_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let code = match codegen::generate(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{cfg_path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match args.get(1) {
        Some(out_path) => {
            if let Err(e) = std::fs::write(out_path, code) {
                eprintln!("{out_path}: {e}");
                return ExitCode::FAILURE;
            }
        }
        None => print!("{code}"),
    }
    ExitCode::SUCCESS
}

fn validate(args: &[String]) -> ExitCode {
    let Some(cfg_path) = args.first() else {
        print_usage();
        return ExitCode::FAILURE;
    };
    let config = match read_config(cfg_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match codegen::generate(&config) {
        Ok(_) => {
            println!(
                "{cfg_path}: OK ({} task(s), {} os_application(s))",
                config.tasks.len(),
                config.os_applications.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{cfg_path}: {e}");
            ExitCode::FAILURE
        }
    }
}
