//! Terminal setup/teardown and the event loop. Translates real
//! `crossterm` key events into [`crate::app::Key`] and drives [`App`];
//! rendering itself is `ui::draw`. Deliberately thin - all the logic
//! that's worth unit-testing lives in `app.rs` instead.

use crate::app::{App, Key};
use crate::ui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

pub fn run(mut app: App) -> io::Result<()> {
    let mut terminal = setup()?;
    let result = event_loop(&mut terminal, &mut app);
    teardown(&mut terminal)?;
    result
}

fn setup() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn teardown(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if let Some(mapped) = map_key(key.code) {
                app.handle_key(mapped);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn map_key(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Char(c) => Some(Key::Char(c)),
        _ => None,
    }
}
