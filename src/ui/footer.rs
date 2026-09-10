use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::text::refreshed;
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
            format!("/{query}  {visible}/{total}")
        }
        (None, Some(at)) => refreshed(at, app.now, app.utc_offset_secs),
        (None, None) => String::new(),
    }
}

pub fn logo() -> String {
    format!(
        "\u{27e6}\u{25a3}\u{27e7}\u{2501}\u{27e6}\u{25a3}\u{27e7}\u{2501}\u{27e6}\u{25a3}\u{27e7}\u{2501}\u{25b8} conveyor v{}",
        env!("CARGO_PKG_VERSION")
    )
}

pub(crate) fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.any_refreshing() {
        format!("{} {}", app.spinner(), footer_text(app))
    } else {
        footer_text(app)
    };
    let logo = logo();
    let logo_width = logo.chars().count() as u16 + 1;
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(logo_width)]).areas(area);
    frame.render_widget(Paragraph::new(text), left);
    frame.render_widget(
        Paragraph::new(Span::styled(logo, dim())).right_aligned(),
        right,
    );
}
