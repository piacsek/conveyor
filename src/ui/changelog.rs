use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::changelog::CHANGELOG;
use crate::ui::details::scrolled;

pub(crate) fn draw_changelog(frame: &mut Frame, app: &mut App, area: Rect) {
    let (page, (scroll, arrows)) = scrolled(area, CHANGELOG.lines().count(), app.changelog_scroll);
    app.changelog_page = page;
    app.changelog_scroll = scroll;
    frame.render_widget(
        Paragraph::new(CHANGELOG)
            .scroll((scroll, 0))
            .block(Block::bordered().title(format!("Changelog{arrows}"))),
        area,
    );
}
