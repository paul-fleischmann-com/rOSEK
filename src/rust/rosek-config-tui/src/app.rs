//! Editor state and pure editing logic — no terminal/rendering code here
//! on purpose, so this module is unit-testable without a real terminal
//! (see `tests/` in this crate). `main.rs`/`tui.rs` translate real
//! `crossterm` key events into [`Key`] and call into [`App`]; `ui.rs`
//! renders an [`App`] to a `ratatui` frame. Neither of those needs to
//! change for the editing logic itself to be tested.
//!
//! ## Extending with a new block type
//!
//! Adding e.g. `RESOURCE` once `rosek-config`'s grammar grows one: add a
//! `Focus::Resources` variant, a `ResourceForm`, a `Mode::EditResource`
//! variant, and mirror what [`Focus::Tasks`]/[`TaskForm`]/
//! `Mode::EditTask` do in `next`/`previous`/`start_edit_selected`/
//! `start_new`/`delete_selected`/`confirm_edit`. Same pattern each time.

use rosek_config::{codegen, serialize, Config, OsApplication, Schedule, Task};
use std::path::PathBuf;

/// Terminal-library-agnostic key input. `tui.rs` maps `crossterm` events
/// onto this so `App`'s logic can be tested without a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Tab,
    Enter,
    Esc,
    Backspace,
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tasks,
    OsApplications,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskForm {
    pub name: String,
    pub priority: String,
    pub schedule: Schedule,
    pub application: String,
    pub field: usize,
}

impl TaskForm {
    const FIELD_COUNT: usize = 4;

    fn from_task(task: &Task) -> Self {
        TaskForm {
            name: task.name.clone(),
            priority: task.priority.to_string(),
            schedule: task.schedule,
            application: task.application.clone(),
            field: 0,
        }
    }

    fn blank() -> Self {
        TaskForm {
            name: String::new(),
            priority: "0".to_string(),
            schedule: Schedule::Full,
            application: String::new(),
            field: 0,
        }
    }

    /// `None` if `priority` isn't a valid `u32` - surfaced to the user as
    /// a status message rather than silently discarding the edit.
    fn try_into_task(&self) -> Result<Task, String> {
        let priority: u32 = self
            .priority
            .parse()
            .map_err(|_| format!("PRIORITY '{}' is not a valid number", self.priority))?;
        if self.name.is_empty() {
            return Err("TASK name must not be empty".to_string());
        }
        if self.application.is_empty() {
            return Err("APPLICATION must not be empty".to_string());
        }
        Ok(Task {
            name: self.name.clone(),
            priority,
            schedule: self.schedule,
            application: self.application.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsAppForm {
    pub name: String,
    pub trusted: bool,
    pub field: usize,
}

impl OsAppForm {
    const FIELD_COUNT: usize = 2;

    fn from_app(app: &OsApplication) -> Self {
        OsAppForm {
            name: app.name.clone(),
            trusted: app.trusted,
            field: 0,
        }
    }

    fn blank() -> Self {
        OsAppForm {
            name: String::new(),
            trusted: false,
            field: 0,
        }
    }

    fn try_into_app(&self) -> Result<OsApplication, String> {
        if self.name.is_empty() {
            return Err("OS_APPLICATION name must not be empty".to_string());
        }
        Ok(OsApplication {
            name: self.name.clone(),
            trusted: self.trusted,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    View,
    EditTask {
        form: TaskForm,
        is_new: bool,
        index: usize,
    },
    EditOsApplication {
        form: OsAppForm,
        is_new: bool,
        index: usize,
    },
}

pub struct App {
    pub config: Config,
    pub path: PathBuf,
    pub focus: Focus,
    pub selected: usize,
    pub mode: Mode,
    pub dirty: bool,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(path: PathBuf, config: Config) -> Self {
        App {
            config,
            path,
            focus: Focus::Tasks,
            selected: 0,
            mode: Mode::View,
            dirty: false,
            status: String::new(),
            should_quit: false,
        }
    }

    fn list_len(&self) -> usize {
        match self.focus {
            Focus::Tasks => self.config.tasks.len(),
            Focus::OsApplications => self.config.os_applications.len(),
        }
    }

    /// The live codegen preview, or the error `rosek-config` would raise
    /// (e.g. a TASK referencing an unknown OS_APPLICATION) - shown as-is
    /// in the preview pane instead of crashing the editor.
    pub fn preview(&self) -> Result<String, String> {
        codegen::generate(&self.config).map_err(|e| e.to_string())
    }

    pub fn handle_key(&mut self, key: Key) {
        match &mut self.mode {
            Mode::View => self.handle_view_key(key),
            Mode::EditTask { .. } | Mode::EditOsApplication { .. } => self.handle_edit_key(key),
        }
    }

    fn handle_view_key(&mut self, key: Key) {
        match key {
            Key::Char('q') => self.should_quit = true,
            Key::Tab => {
                self.focus = match self.focus {
                    Focus::Tasks => Focus::OsApplications,
                    Focus::OsApplications => Focus::Tasks,
                };
                self.selected = 0;
            }
            Key::Down => self.next(),
            Key::Up => self.previous(),
            Key::Enter => self.start_edit_selected(),
            Key::Char('n') => self.start_new(),
            Key::Char('x') => self.delete_selected(),
            Key::Char('s') => self.save(),
            _ => {}
        }
    }

    fn next(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    fn previous(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected + len - 1) % len;
    }

    fn start_edit_selected(&mut self) {
        match self.focus {
            Focus::Tasks => {
                if let Some(task) = self.config.tasks.get(self.selected) {
                    self.mode = Mode::EditTask {
                        form: TaskForm::from_task(task),
                        is_new: false,
                        index: self.selected,
                    };
                }
            }
            Focus::OsApplications => {
                if let Some(app) = self.config.os_applications.get(self.selected) {
                    self.mode = Mode::EditOsApplication {
                        form: OsAppForm::from_app(app),
                        is_new: false,
                        index: self.selected,
                    };
                }
            }
        }
    }

    fn start_new(&mut self) {
        match self.focus {
            Focus::Tasks => {
                self.mode = Mode::EditTask {
                    form: TaskForm::blank(),
                    is_new: true,
                    index: self.config.tasks.len(),
                };
            }
            Focus::OsApplications => {
                self.mode = Mode::EditOsApplication {
                    form: OsAppForm::blank(),
                    is_new: true,
                    index: self.config.os_applications.len(),
                };
            }
        }
    }

    fn delete_selected(&mut self) {
        match self.focus {
            Focus::Tasks => {
                if self.selected < self.config.tasks.len() {
                    self.config.tasks.remove(self.selected);
                    self.dirty = true;
                }
            }
            Focus::OsApplications => {
                if self.selected < self.config.os_applications.len() {
                    self.config.os_applications.remove(self.selected);
                    self.dirty = true;
                }
            }
        }
        let len = self.list_len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    fn handle_edit_key(&mut self, key: Key) {
        if key == Key::Esc {
            self.mode = Mode::View;
            return;
        }
        if key == Key::Enter {
            self.confirm_edit();
            return;
        }
        match &mut self.mode {
            Mode::EditTask { form, .. } => Self::handle_task_form_key(form, key),
            Mode::EditOsApplication { form, .. } => Self::handle_os_app_form_key(form, key),
            Mode::View => {}
        }
    }

    fn handle_task_form_key(form: &mut TaskForm, key: Key) {
        match key {
            Key::Tab | Key::Down => form.field = (form.field + 1) % TaskForm::FIELD_COUNT,
            Key::Up => {
                form.field = (form.field + TaskForm::FIELD_COUNT - 1) % TaskForm::FIELD_COUNT
            }
            Key::Left | Key::Right if form.field == 1 => {
                form.schedule = match form.schedule {
                    Schedule::Full => Schedule::Non,
                    Schedule::Non => Schedule::Full,
                };
            }
            Key::Char(c) => text_field_mut(form, |s| s.push(c)),
            Key::Backspace => text_field_mut(form, |s| {
                s.pop();
            }),
            _ => {}
        }
    }

    fn handle_os_app_form_key(form: &mut OsAppForm, key: Key) {
        match key {
            Key::Tab | Key::Down => form.field = (form.field + 1) % OsAppForm::FIELD_COUNT,
            Key::Up => {
                form.field = (form.field + OsAppForm::FIELD_COUNT - 1) % OsAppForm::FIELD_COUNT
            }
            Key::Left | Key::Right if form.field == 1 => form.trusted = !form.trusted,
            Key::Char(c) if form.field == 0 => form.name.push(c),
            Key::Backspace if form.field == 0 => {
                form.name.pop();
            }
            _ => {}
        }
    }

    fn confirm_edit(&mut self) {
        match std::mem::replace(&mut self.mode, Mode::View) {
            Mode::EditTask {
                form,
                is_new,
                index,
            } => match form.try_into_task() {
                Ok(task) => {
                    if is_new {
                        self.config.tasks.push(task);
                    } else {
                        self.config.tasks[index] = task;
                    }
                    self.dirty = true;
                    self.status.clear();
                }
                Err(e) => {
                    self.status = e;
                    self.mode = Mode::EditTask {
                        form,
                        is_new,
                        index,
                    };
                }
            },
            Mode::EditOsApplication {
                form,
                is_new,
                index,
            } => match form.try_into_app() {
                Ok(app) => {
                    if is_new {
                        self.config.os_applications.push(app);
                    } else {
                        self.config.os_applications[index] = app;
                    }
                    self.dirty = true;
                    self.status.clear();
                }
                Err(e) => {
                    self.status = e;
                    self.mode = Mode::EditOsApplication {
                        form,
                        is_new,
                        index,
                    };
                }
            },
            Mode::View => {}
        }
    }

    pub fn save(&mut self) {
        let text = serialize::to_cfg_string(&self.config);
        match std::fs::write(&self.path, text) {
            Ok(()) => {
                self.dirty = false;
                self.status = format!("saved to {}", self.path.display());
            }
            Err(e) => self.status = format!("save failed: {e}"),
        }
    }
}

/// Only fields 0 (name) and 2 (application) of [`TaskForm`] are free text;
/// field 1 (schedule) is toggled via Left/Right, field 3 is a text field
/// too (priority) reusing the same digit-entry path as name/application -
/// still just character push/pop, validated as a number on confirm.
fn text_field_mut(form: &mut TaskForm, f: impl FnOnce(&mut String)) {
    match form.field {
        0 => f(&mut form.name),
        2 => f(&mut form.application),
        3 => f(&mut form.priority),
        _ => {}
    }
}
