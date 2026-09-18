//! Integration tests for `rosek_config::serialize::to_cfg_string`.

use rosek_config::serialize::to_cfg_string;
use rosek_config::{parse, Config, OsApplication, Schedule, Task};

fn sample_config() -> Config {
    Config {
        tasks: vec![
            Task {
                name: "MainTask".into(),
                priority: 5,
                schedule: Schedule::Full,
                application: "TrustedApp".into(),
            },
            Task {
                name: "BgTask".into(),
                priority: 0,
                schedule: Schedule::Non,
                application: "Untrusted".into(),
            },
        ],
        os_applications: vec![
            OsApplication {
                name: "TrustedApp".into(),
                trusted: true,
            },
            OsApplication {
                name: "Untrusted".into(),
                trusted: false,
            },
        ],
    }
}

#[test]
fn serialized_output_parses_back_to_an_equal_config() {
    let config = sample_config();
    let text = to_cfg_string(&config);
    let reparsed = parse(&text).expect("serialized output must re-parse");
    assert_eq!(reparsed, config);
}

#[test]
fn empty_config_serializes_to_empty_string() {
    assert_eq!(to_cfg_string(&Config::default()), "");
}

#[test]
fn output_uses_the_expected_grammar_tokens() {
    let text = to_cfg_string(&sample_config());
    assert!(text.contains("OS_APPLICATION TrustedApp {"));
    assert!(text.contains("TRUSTED = TRUE;"));
    assert!(text.contains("TASK MainTask {"));
    assert!(text.contains("PRIORITY = 5;"));
    assert!(text.contains("SCHEDULE = FULL;"));
    assert!(text.contains("APPLICATION = TrustedApp;"));
}
