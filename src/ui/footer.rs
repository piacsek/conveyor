use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::text::{cells, refreshed};
use crate::ui::style::dim;

pub(crate) fn footer_text(app: &App) -> String {
    if let Some(notice) = &app.notice {
        return notice.clone();
    }
    if let Some(error) = app.focused_error() {
        return error.to_string();
    }
    match (app.filter(), app.focused_fetched_at()) {
        (Some(query), _) => {
            let (visible, total) = app.focused_counts();
            let cursor = if app.editing_filter() { "▏" } else { "" };
            format!("/{query}{cursor}  {visible}/{total}")
        }
        (None, Some(at)) => refreshed(at, app.now, app.utc_offset_secs),
        (None, None) => String::new(),
    }
}

const DEMO: &str = "demo";

pub fn logo() -> String {
    format!(
        "\u{25aa}\u{2500}\u{25aa}\u{2500}\u{25aa}\u{25b8} conveyor v{}",
        env!("CARGO_PKG_VERSION")
    )
}

pub(crate) fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.any_refreshing() {
        format!("{} {}", app.spinner(), footer_text(app))
    } else {
        footer_text(app)
    };
    let left = draw_right(frame, app, Some(Span::styled(logo(), dim())), area);
    frame.render_widget(Paragraph::new(text), left);
}

pub(crate) fn draw_overlay_footer(frame: &mut Frame, app: &App, hint: &str, area: Rect) {
    let text = app.notice.as_deref().unwrap_or(hint);
    let left = draw_right(frame, app, None, area);
    frame.render_widget(Paragraph::new(text.to_string()), left);
}

fn draw_right(frame: &mut Frame, app: &App, logo: Option<Span<'static>>, area: Rect) -> Rect {
    let mut spans = Vec::new();
    if app.demo {
        spans.push(Span::styled(
            DEMO,
            Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(logo) = logo {
        if app.demo {
            spans.push(Span::raw(" "));
        }
        spans.push(logo);
    }
    let line = Line::from(spans);
    let width = cells(line.width());
    let gap = u16::from(width > 0);
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width + gap)]).areas(area);
    frame.render_widget(Paragraph::new(line).right_aligned(), right);
    left
}
