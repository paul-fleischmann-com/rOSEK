//! CLI + interactive TUI editor for rOSEK's `.cfg` static configuration
//! files (see `rosek-config`'s grammar doc-comment and
//! `docs/arc42/08-concepts.adoc`). Dev tooling only - not part of the
//! kernel, see `src/rust/Cargo.toml` for why this crate lives outside
//! the main workspace.
//!
//! ```text
//! rosek-config-tui generate <path.cfg>            print generated Rust to stdout
//! rosek-config-tui generate <path.cfg> <out.rs>    write generated Rust to a file
//! rosek-config-tui validate <path.cfg>             parse + validate, print OK or the error
//! rosek-config-tui edit <path.cfg>                 interactive terminal editor
//! ```

mod app;
mod tui;
mod ui;

use app::App;
use rosek_config::{codegen, parse, Config};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("generate") => generate(&args[1..]),
        Some("validate") => validate(&args[1..]),
        Some("edit") => edit(&args[1..]),
        _ => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!(
        "usage:\n  \
         rosek-config-tui generate <path.cfg> [out.rs]\n  \
         rosek-config-tui validate <path.cfg>\n  \
         rosek-config-tui edit <path.cfg>"
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

fn edit(args: &[String]) -> ExitCode {
    let Some(cfg_path) = args.first() else {
        print_usage();
        return ExitCode::FAILURE;
    };
    let path = PathBuf::from(cfg_path);
    let config = if path.exists() {
        match read_config(cfg_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        Config::default()
    };

    let app = App::new(path, config);
    if let Err(e) = tui::run(app) {
        eprintln!("terminal error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
