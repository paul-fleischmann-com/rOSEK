//! Integration tests for `rosek_config::parse` — public API only, no
//! access to the crate's internals (lexer/parser structs stay private).
//! See `tests/codegen.rs` for the code-generation side.

use rosek_config::{parse, Config, OsApplication, Schedule, Task};

#[test]
fn parses_the_example_from_the_grammar_decision() {
    let src = r#"
        TASK MainTask {
          PRIORITY = 5;
          SCHEDULE = FULL;
          APPLICATION = TrustedApp;
        };

        OS_APPLICATION TrustedApp {
          TRUSTED = TRUE;
        };
    "#;
    let config = parse(src).expect("should parse");
    assert_eq!(
        config.tasks,
        vec![Task {
            name: "MainTask".into(),
            priority: 5,
            schedule: Schedule::Full,
            application: "TrustedApp".into(),
        }]
    );
    assert_eq!(
        config.os_applications,
        vec![OsApplication {
            name: "TrustedApp".into(),
            trusted: true,
        }]
    );
}

#[test]
fn parses_non_preemptive_and_non_trusted() {
    let src = r#"
        TASK Bg { PRIORITY = 0; SCHEDULE = NON; APPLICATION = Untrusted; };
        OS_APPLICATION Untrusted { TRUSTED = FALSE; };
    "#;
    let config = parse(src).unwrap();
    assert_eq!(config.tasks[0].schedule, Schedule::Non);
    assert!(!config.os_applications[0].trusted);
}

#[test]
fn skips_line_comments() {
    let src = r#"
        // this is a comment
        TASK T { // trailing comment
          PRIORITY = 1; // another one
          SCHEDULE = FULL;
          APPLICATION = A;
        };
    "#;
    let config = parse(src).unwrap();
    assert_eq!(config.tasks.len(), 1);
}

#[test]
fn empty_file_parses_to_empty_config() {
    assert_eq!(parse("").unwrap(), Config::default());
    assert_eq!(
        parse("   \n // just a comment\n").unwrap(),
        Config::default()
    );
}

#[test]
fn multiple_blocks() {
    let src = r#"
        TASK A { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = App1; };
        TASK B { PRIORITY = 2; SCHEDULE = NON; APPLICATION = App2; };
        OS_APPLICATION App1 { TRUSTED = TRUE; };
        OS_APPLICATION App2 { TRUSTED = FALSE; };
    "#;
    let config = parse(src).unwrap();
    assert_eq!(config.tasks.len(), 2);
    assert_eq!(config.os_applications.len(), 2);
}

#[test]
fn rejects_unknown_top_level_keyword() {
    let err = parse("RESOURCE Foo { };").unwrap_err();
    assert!(err.message.contains("TASK or OS_APPLICATION"));
}

#[test]
fn rejects_unknown_task_attribute() {
    let err = parse("TASK T { FOO = 1; };").unwrap_err();
    assert!(err.message.contains("unknown TASK attribute"));
}

#[test]
fn rejects_invalid_schedule_value() {
    let err = parse("TASK T { PRIORITY = 1; SCHEDULE = MAYBE; APPLICATION = A; };").unwrap_err();
    assert!(err.message.contains("FULL or NON"));
}

#[test]
fn rejects_missing_required_attribute() {
    let err = parse("TASK T { PRIORITY = 1; SCHEDULE = FULL; };").unwrap_err();
    assert!(err.message.contains("missing APPLICATION"));
}

#[test]
fn rejects_missing_semicolon_after_attribute() {
    let err = parse("TASK T { PRIORITY = 1 };").unwrap_err();
    assert!(err.message.contains("';'"));
}

#[test]
fn rejects_missing_semicolon_after_block() {
    let err = parse("TASK T { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = A; }").unwrap_err();
    assert!(err.message.contains("';' after block"));
}

#[test]
fn error_reports_correct_line_number() {
    let src = "TASK T {\n  PRIORITY = 1;\n  FOO = 2;\n};";
    let err = parse(src).unwrap_err();
    assert_eq!(err.line, 3);
}

#[test]
fn priority_must_be_a_number() {
    let err = parse("TASK T { PRIORITY = FULL; SCHEDULE = FULL; APPLICATION = A; };").unwrap_err();
    assert!(err.message.contains("PRIORITY must be a number"));
}

#[test]
fn application_must_be_an_identifier() {
    let err = parse("TASK T { PRIORITY = 1; SCHEDULE = FULL; APPLICATION = 42; };").unwrap_err();
    assert!(err.message.contains("APPLICATION must be an identifier"));
}

/// Regression test: the lexer used to silently clamp an overflowing digit
/// run to `u32::MAX` and hand the parser a *valid*-looking number token,
/// so a `.cfg` with e.g. `PRIORITY = 99999999999;` would parse
/// successfully with a wrong value instead of failing. Fixed by emitting
/// overflowing digit runs as an identifier instead, which correctly fails
/// value-type checking.
#[test]
fn rejects_priority_that_overflows_u32() {
    let err =
        parse("TASK T { PRIORITY = 99999999999; SCHEDULE = FULL; APPLICATION = A; };").unwrap_err();
    assert!(err.message.contains("PRIORITY must be a number"));
}

#[test]
fn tabs_and_crlf_are_whitespace() {
    let src = "TASK\tT\t{\r\n\tPRIORITY\t=\t1;\r\n\tSCHEDULE = FULL;\r\n\tAPPLICATION = A;\r\n};";
    let config = parse(src).unwrap();
    assert_eq!(config.tasks[0].name, "T");
}
