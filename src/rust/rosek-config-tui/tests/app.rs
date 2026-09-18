//! Unit tests for the editing logic in `src/app.rs`. Drives `App` purely
//! through `Key` values - no terminal, no `ratatui`/`crossterm` needed,
//! which is the whole point of keeping that module free of both.

use rosek_config::{Config, OsApplication, Schedule, Task};
use std::path::PathBuf;

#[path = "../src/app.rs"]
mod app;
use app::{App, Focus, Key, Mode};

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

fn type_str(app: &mut App, s: &str) {
    for c in s.chars() {
        app.handle_key(Key::Char(c));
    }
}

#[test]
fn tab_switches_focus_between_tasks_and_os_applications() {
    let mut app = App::new(PathBuf::from("x.cfg"), sample_config());
    assert_eq!(app.focus, Focus::Tasks);
    app.handle_key(Key::Tab);
    assert_eq!(app.focus, Focus::OsApplications);
    app.handle_key(Key::Tab);
    assert_eq!(app.focus, Focus::Tasks);
}

#[test]
fn up_down_wrap_around_the_selected_list() {
    let mut config = sample_config();
    config.tasks.push(Task {
        name: "BgTask".into(),
        priority: 1,
        schedule: Schedule::Non,
        application: "TrustedApp".into(),
    });
    let mut app = App::new(PathBuf::from("x.cfg"), config);
    assert_eq!(app.selected, 0);
    app.handle_key(Key::Up);
    assert_eq!(app.selected, 1, "up from index 0 wraps to the last item");
    app.handle_key(Key::Down);
    assert_eq!(app.selected, 0);
}

#[test]
fn new_task_is_added_after_filling_the_form_and_confirming() {
    let mut app = App::new(PathBuf::from("x.cfg"), sample_config());
    app.handle_key(Key::Char('n'));
    assert!(matches!(app.mode, Mode::EditTask { is_new: true, .. }));

    type_str(&mut app, "NewTask");
    app.handle_key(Key::Tab); // -> schedule field
    app.handle_key(Key::Tab); // -> application field
    type_str(&mut app, "TrustedApp");
    app.handle_key(Key::Tab); // -> priority field
    type_str(&mut app, "7");
    app.handle_key(Key::Enter);

    assert!(matches!(app.mode, Mode::View));
    assert_eq!(app.config.tasks.len(), 2);
    let added = &app.config.tasks[1];
    assert_eq!(added.name, "NewTask");
    assert_eq!(added.priority, 7);
    assert_eq!(added.schedule, Schedule::Full);
    assert_eq!(added.application, "TrustedApp");
    assert!(app.dirty);
}

#[test]
fn invalid_priority_keeps_the_form_open_with_a_status_message() {
    let mut app = App::new(PathBuf::from("x.cfg"), sample_config());
    app.handle_key(Key::Char('n'));
    type_str(&mut app, "Bad");
    app.handle_key(Key::Tab);
    app.handle_key(Key::Tab);
    type_str(&mut app, "TrustedApp");
    app.handle_key(Key::Tab);
    type_str(&mut app, "not-a-number");
    app.handle_key(Key::Enter);

    assert!(
        matches!(app.mode, Mode::EditTask { .. }),
        "form must stay open on invalid input"
    );
    assert!(!app.status.is_empty());
    assert_eq!(app.config.tasks.len(), 1, "no task was added");
}

#[test]
fn escape_cancels_a_new_item_without_adding_it() {
    let mut app = App::new(PathBuf::from("x.cfg"), sample_config());
    app.handle_key(Key::Char('n'));
    type_str(&mut app, "Discarded");
    app.handle_key(Key::Esc);

    assert!(matches!(app.mode, Mode::View));
    assert_eq!(app.config.tasks.len(), 1);
}

#[test]
fn editing_an_existing_task_updates_it_in_place() {
    let mut app = App::new(PathBuf::from("x.cfg"), sample_config());
    app.handle_key(Key::Enter); // edit the selected (only) task
                                // field 0 = name: clear it out, then retype
    for _ in 0.."MainTask".len() {
        app.handle_key(Key::Backspace);
    }
    type_str(&mut app, "RenamedTask");
    app.handle_key(Key::Enter);

    assert_eq!(app.config.tasks.len(), 1);
    assert_eq!(app.config.tasks[0].name, "RenamedTask");
}

#[test]
fn delete_removes_the_selected_task_and_clamps_selection() {
    let mut config = sample_config();
    config.tasks.push(Task {
        name: "BgTask".into(),
        priority: 1,
        schedule: Schedule::Non,
        application: "TrustedApp".into(),
    });
    let mut app = App::new(PathBuf::from("x.cfg"), config);
    app.handle_key(Key::Down); // select the 2nd task
    assert_eq!(app.selected, 1);
    app.handle_key(Key::Char('x'));
    assert_eq!(app.config.tasks.len(), 1);
    assert_eq!(app.selected, 0, "selection clamped back into range");
    assert!(app.dirty);
}

#[test]
fn schedule_and_trusted_fields_toggle_instead_of_accepting_free_text() {
    let mut app = App::new(PathBuf::from("x.cfg"), sample_config());
    app.handle_key(Key::Enter); // edit MainTask
    app.handle_key(Key::Tab); // -> schedule field
    if let Mode::EditTask { form, .. } = &app.mode {
        assert_eq!(form.schedule, Schedule::Full);
    } else {
        panic!("expected EditTask mode");
    }
    app.handle_key(Key::Right);
    if let Mode::EditTask { form, .. } = &app.mode {
        assert_eq!(form.schedule, Schedule::Non);
    } else {
        panic!("expected EditTask mode");
    }
    app.handle_key(Key::Left);
    if let Mode::EditTask { form, .. } = &app.mode {
        assert_eq!(
            form.schedule,
            Schedule::Full,
            "Left toggles back just like Right"
        );
    } else {
        panic!("expected EditTask mode");
    }
}

#[test]
fn preview_reflects_a_reference_to_an_undeclared_application() {
    let mut app = App::new(PathBuf::from("x.cfg"), sample_config());
    app.handle_key(Key::Enter); // edit MainTask, field starts at 0 (name)
    app.handle_key(Key::Tab); // -> schedule
    app.handle_key(Key::Tab); // -> application
    for _ in 0.."TrustedApp".len() {
        app.handle_key(Key::Backspace);
    }
    type_str(&mut app, "Ghost");
    app.handle_key(Key::Enter);

    assert!(
        app.preview().is_err(),
        "Ghost is not a declared OS_APPLICATION"
    );
}

#[test]
fn save_writes_the_serialized_config_to_disk() {
    let dir = std::env::temp_dir().join(format!("rosek-config-tui-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.cfg");

    let mut app = App::new(path.clone(), sample_config());
    app.save();

    assert!(!app.dirty);
    let written = std::fs::read_to_string(&path).unwrap();
    let reparsed = rosek_config::parse(&written).unwrap();
    assert_eq!(reparsed, sample_config());

    std::fs::remove_dir_all(&dir).ok();
}
