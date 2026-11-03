mod card;
pub mod changelog;
mod columns;
mod details;
mod dialog;
mod footer;
pub mod help;
mod rows;
mod style;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, Mode, Stage};
use crate::ui::changelog::draw_changelog;
use crate::ui::columns::{column_title, draw_column};
use crate::ui::details::draw_details;
use crate::ui::dialog::{draw_confirm, draw_manage};
use crate::ui::footer::draw_footer;
use crate::ui::help::draw_help;
use crate::ui::style::dim;

pub use crate::ui::footer::logo;
pub use crate::ui::help::KEYS;

fn draw_overlay_footer(frame: &mut Frame, app: &App, hint: &str, area: Rect) {
    let text = app.notice.as_deref().unwrap_or(hint);
    frame.render_widget(Paragraph::new(text.to_string()), area);
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, footer] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    if app.mode == Mode::Changelog {
        draw_changelog(frame, app, body);
        draw_overlay_footer(
            frame,
            app,
            "j/k C-d/C-u scroll · any other key returns",
            footer,
        );
        return;
    }
    if app.mode == Mode::Help {
        draw_help(frame, body, app.focus);
        draw_overlay_footer(frame, app, "any key returns", footer);
        return;
    }
    let whole = body;
    let body = if app.details {
        let details_percent = app.config.ui.details_percent;
        let [columns, details] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(details_percent)])
                .areas(body);
        draw_details(frame, app, details);
        columns
    } else {
        body
    };
    draw_footer(frame, app, footer);
    if app.zoom {
        draw_column(frame, app, app.focus, body, true);
    } else if body.width < app.config.ui.min_column_width * Stage::ALL.len() as u16 {
        draw_tabs(frame, app, body);
    } else {
        let columns = Layout::horizontal([Constraint::Fill(1); 4]).split(body);
        for (stage, area) in Stage::ALL.into_iter().zip(columns.iter()) {
            draw_column(frame, app, stage, *area, true);
        }
    }
    if let Mode::Confirm(pending) = &app.mode {
        draw_confirm(frame, whole, pending);
    }
    if let Mode::Manage(menu) = &app.mode {
        draw_manage(frame, whole, menu);
    }
}

fn draw_tabs(frame: &mut Frame, app: &mut App, area: Rect) {
    let [bar, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
    let mut spans = Vec::new();
    for (i, stage) in Stage::ALL.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" │ "));
        }
        let style = if stage == app.focus {
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            dim()
        };
        spans.push(Span::styled(column_title(app, stage), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), bar);
    draw_column(frame, app, app.focus, body, false);
}
