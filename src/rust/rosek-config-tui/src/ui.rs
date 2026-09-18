//! Renders an [`App`] onto a `ratatui` frame. Pure presentation - all
//! state lives in `app.rs` and none of this is invoked from the unit
//! tests, since it can't be checked without an actual terminal backend.

use crate::app::{App, Focus, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(frame.area());

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[0]);

    draw_lists(frame, app, columns[0]);
    draw_preview(frame, app, columns[1]);
    draw_status(frame, app, root[1]);

    if let Mode::EditTask { .. } | Mode::EditOsApplication { .. } = &app.mode {
        draw_edit_popup(frame, app, frame.area());
    }
}

fn draw_lists(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let task_items: Vec<ListItem> = app
        .config
        .tasks
        .iter()
        .map(|t| {
            ListItem::new(format!(
                "{} (prio {}, {:?}, app={})",
                t.name, t.priority, t.schedule, t.application
            ))
        })
        .collect();
    let task_selected = app.focus == Focus::Tasks;
    frame.render_widget(
        List::new(task_items)
            .block(highlighted_block("TASKS", task_selected))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        rows[0],
    );

    let app_items: Vec<ListItem> = app
        .config
        .os_applications
        .iter()
        .map(|a| ListItem::new(format!("{} (trusted={})", a.name, a.trusted)))
        .collect();
    let apps_selected = app.focus == Focus::OsApplications;
    frame.render_widget(
        List::new(app_items)
            .block(highlighted_block("OS_APPLICATIONS", apps_selected))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        rows[1],
    );
}

fn highlighted_block(title: &str, selected: bool) -> Block<'_> {
    let style = if selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style)
}

fn draw_preview(frame: &mut Frame, app: &App, area: Rect) {
    let text = match app.preview() {
        Ok(code) => code,
        Err(e) => format!("codegen error:\n{e}"),
    };
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title("GENERATED PREVIEW")
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let dirty = if app.dirty { " [modified]" } else { "" };
    let help =
        "Tab: switch list | ↑/↓: select | Enter: edit | n: new | x: delete | s: save | q: quit";
    let line = if app.status.is_empty() {
        Line::from(Span::raw(format!("{help}{dirty}")))
    } else {
        Line::from(Span::styled(
            app.status.clone(),
            Style::default().fg(Color::Red),
        ))
    };
    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.path.display().to_string()),
        ),
        area,
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn draw_edit_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered(area, 60, 10);
    let task_schedule_text;
    let lines: Vec<Line> = match &app.mode {
        Mode::EditTask { form, .. } => {
            task_schedule_text = format!("{:?}", form.schedule);
            vec![
                field_line("NAME", &form.name, form.field == 0),
                field_line("SCHEDULE", &task_schedule_text, form.field == 1),
                field_line("APPLICATION", &form.application, form.field == 2),
                field_line("PRIORITY", &form.priority, form.field == 3),
            ]
        }
        Mode::EditOsApplication { form, .. } => vec![
            field_line("NAME", &form.name, form.field == 0),
            field_line(
                "TRUSTED",
                if form.trusted { "TRUE" } else { "FALSE" },
                form.field == 1,
            ),
        ],
        Mode::View => vec![],
    };
    let title = match &app.mode {
        Mode::EditTask { is_new: true, .. } => {
            "New TASK (Tab: next field, Enter: save, Esc: cancel)"
        }
        Mode::EditTask { is_new: false, .. } => {
            "Edit TASK (Tab: next field, Enter: save, Esc: cancel)"
        }
        Mode::EditOsApplication { is_new: true, .. } => {
            "New OS_APPLICATION (Tab: next field, Enter: save, Esc: cancel)"
        }
        Mode::EditOsApplication { is_new: false, .. } => {
            "Edit OS_APPLICATION (Tab: next field, Enter: save, Esc: cancel)"
        }
        Mode::View => "",
    };
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().title(title).borders(Borders::ALL)),
        popup,
    );
}

fn field_line<'a>(label: &'a str, value: &'a str, focused: bool) -> Line<'a> {
    let style = if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    Line::from(Span::styled(format!("{label}: {value}"), style))
}
