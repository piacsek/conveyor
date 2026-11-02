use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crate::app::{App, Mode, Pending};
use crate::text::{refreshed, truncate};
use crate::ui::style::dim;

pub(crate) fn footer_text(app: &App, width: usize) -> String {
    if let Mode::Confirm(pending) = &app.mode {
        return match pending {
            Pending::Rerun(rerun) => format!("rerun failed jobs of {}? y/n", rerun.label),
            Pending::Merge(target) => {
                let prefix = format!("merge #{} ", target.number);
                let room = width.saturating_sub(prefix.chars().count() + "? y/n".len());
                format!("{prefix}{}? y/n", truncate(&target.title, room))
            }
        };
    }
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

pub fn logo() -> String {
    format!(
        "\u{25aa}\u{2500}\u{25aa}\u{2500}\u{25aa}\u{25b8} conveyor v{}",
        env!("CARGO_PKG_VERSION")
    )
}

pub(crate) fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let logo = logo();
    let logo_width = logo.chars().count() as u16 + 1;
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(logo_width)]).areas(area);
    let width = usize::from(left.width);
    let text = if app.any_refreshing() {
        format!(
            "{} {}",
            app.spinner(),
            footer_text(app, width.saturating_sub(2))
        )
    } else {
        footer_text(app, width)
    };
    frame.render_widget(Paragraph::new(text), left);
    frame.render_widget(
        Paragraph::new(Span::styled(logo, dim())).right_aligned(),
        right,
    );
}
