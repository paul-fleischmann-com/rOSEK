//! Serialises a [`crate::Config`] back into the OIL-inspired `.cfg` text
//! format (the inverse of [`crate::parse`]). Used by `rosek-config-tui`
//! (interactive editor) to save edits back to a `.cfg` file.
//!
//! **Lossy**: the parser (issue #8) discards comments, so a
//! `parse -> serialize` round-trip reproduces the *data* faithfully but
//! not the original file's comments or exact formatting/whitespace.
//! Acceptable for a config that's meant to be edited through the tool
//! from here on - not meant for a "just re-save unchanged" workflow that
//! must preserve human-written comments.

use crate::{Config, OsApplication, Schedule, Task};
use std::fmt::Write;

/// Renders `config` back into `.cfg` source text.
pub fn to_cfg_string(config: &Config) -> String {
    let mut out = String::new();

    for app in &config.os_applications {
        write_os_application(&mut out, app);
    }
    for task in &config.tasks {
        write_task(&mut out, task);
    }

    out
}

fn write_os_application(out: &mut String, app: &OsApplication) {
    let _ = writeln!(out, "OS_APPLICATION {} {{", app.name);
    let _ = writeln!(
        out,
        "  TRUSTED = {};",
        if app.trusted { "TRUE" } else { "FALSE" }
    );
    let _ = writeln!(out, "}};\n");
}

fn write_task(out: &mut String, task: &Task) {
    let _ = writeln!(out, "TASK {} {{", task.name);
    let _ = writeln!(out, "  PRIORITY = {};", task.priority);
    let schedule = match task.schedule {
        Schedule::Full => "FULL",
        Schedule::Non => "NON",
    };
    let _ = writeln!(out, "  SCHEDULE = {schedule};");
    let _ = writeln!(out, "  APPLICATION = {};", task.application);
    let _ = writeln!(out, "}};\n");
}
