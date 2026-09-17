//! Integration tests for `rosek_config::codegen::generate` — public API
//! only. See `tests/parser.rs` for the parsing side.

use rosek_config::codegen::generate;
use rosek_config::{Config, OsApplication, Schedule, Task};

fn sample_config() -> Config {
    Config {
        tasks: vec![Task {
            name: "MainTask".into(),
            priority: 5,
            schedule: Schedule::Full,
            application: "TrustedApp".into(),
        }],
        os_applications: vec![OsApplication {
            name: "TrustedApp".into(),
            trusted: true,
        }],
    }
}

#[test]
fn generates_valid_looking_rust_source() {
    let src = generate(&sample_config()).unwrap();
    assert!(src.contains("pub static TASKS: &[TaskConfig] = &["));
    assert!(src.contains(r#"name: "MainTask""#));
    assert!(src.contains("priority: 5"));
    assert!(src.contains("schedule: ScheduleConfig::Full"));
    assert!(src.contains(r#"application: "TrustedApp""#));
    assert!(src.contains(r#"name: "TrustedApp", trusted: true"#));
}

#[test]
fn empty_config_generates_empty_arrays() {
    let src = generate(&Config::default()).unwrap();
    assert!(src.contains("pub static TASKS: &[TaskConfig] = &[\n];"));
    assert!(src.contains("pub static OS_APPLICATIONS: &[OsApplicationConfig] = &[\n];"));
}

#[test]
fn os_application_with_no_referencing_task_is_fine() {
    // An OS-Application that no TASK currently references is not an
    // error - only the reverse (a TASK pointing at a nonexistent
    // OS-Application) is.
    let mut config = sample_config();
    config.os_applications.push(OsApplication {
        name: "UnusedApp".into(),
        trusted: false,
    });
    let src = generate(&config).unwrap();
    assert!(src.contains(r#"name: "UnusedApp", trusted: false"#));
}

#[test]
fn generates_multiple_tasks_and_applications_in_declaration_order() {
    let config = Config {
        tasks: vec![
            Task {
                name: "A".into(),
                priority: 1,
                schedule: Schedule::Full,
                application: "App1".into(),
            },
            Task {
                name: "B".into(),
                priority: 2,
                schedule: Schedule::Non,
                application: "App2".into(),
            },
        ],
        os_applications: vec![
            OsApplication {
                name: "App1".into(),
                trusted: true,
            },
            OsApplication {
                name: "App2".into(),
                trusted: false,
            },
        ],
    };
    let src = generate(&config).unwrap();
    let a_pos = src.find(r#"name: "A""#).unwrap();
    let b_pos = src.find(r#"name: "B""#).unwrap();
    assert!(a_pos < b_pos, "tasks should appear in declaration order");
    assert!(src.contains("ScheduleConfig::Non"));
}

#[test]
fn rejects_task_referencing_unknown_application() {
    let mut config = sample_config();
    config.tasks[0].application = "DoesNotExist".into();
    let err = generate(&config).unwrap_err();
    assert!(err.message.contains("DoesNotExist"));
    assert!(err.message.contains("MainTask"));
}

#[test]
fn generated_source_actually_compiles_and_matches_the_config() {
    // Strongest check available here: actually invoke rustc on the
    // generated string and confirm it produces a valid object file, not
    // just "contains the right substrings".
    let src = generate(&sample_config()).unwrap();
    let dir =
        std::env::temp_dir().join(format!("rosek_config_codegen_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src_path = dir.join("generated.rs");
    std::fs::write(&src_path, src).unwrap();

    let out_path = dir.join("generated.o");
    let status = std::process::Command::new("rustc")
        .args(["--crate-type", "lib", "--emit", "obj", "-o"])
        .arg(&out_path)
        .arg(&src_path)
        .status()
        .expect("failed to invoke rustc");
    assert!(status.success(), "generated code did not compile");

    std::fs::remove_dir_all(&dir).ok();
}
