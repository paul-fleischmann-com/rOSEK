//! Interactive terminal editor for rOSEK's `.cfg` static configuration
//! files (see `rosek-config`'s grammar doc-comment and
//! `docs/arc42/08-concepts.adoc`). For scriptable generate/validate
//! (CI/scripting, no TUI dependencies) see `rosek-config-cli` instead.
//! Dev tooling only - not part of the kernel, see `src/rust/Cargo.toml`
//! for why this crate lives outside the main workspace.
//!
//! ```text
//! rosek-config-tui edit <path.cfg>    interactive terminal editor
//! ```

mod app;
mod tui;
mod ui;

use app::App;
use rosek_config::{parse, Config};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("edit") => edit(&args[1..]),
        _ => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!("usage:\n  rosek-config-tui edit <path.cfg>");
}

fn edit(args: &[String]) -> ExitCode {
    let Some(cfg_path) = args.first() else {
        print_usage();
        return ExitCode::FAILURE;
    };
    let path = PathBuf::from(cfg_path);
    let config = if path.exists() {
        match std::fs::read_to_string(&path)
            .map_err(|e| format!("{cfg_path}: {e}"))
            .and_then(|source| parse(&source).map_err(|e| format!("{cfg_path}: {e}")))
        {
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
