use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::changelog::CHANGELOG;
use crate::ui::details::scrolled;

fn line(markdown: &str) -> Line<'_> {
    match markdown
        .strip_prefix("## ")
        .or_else(|| markdown.strip_prefix("# "))
    {
        Some(heading) => Line::styled(heading, Style::default().add_modifier(Modifier::BOLD)),
        None => Line::raw(markdown),
    }
}

pub(crate) fn draw_changelog(frame: &mut Frame, app: &mut App, area: Rect) {
    let lines: Vec<Line> = CHANGELOG.lines().map(line).collect();
    let (page, (scroll, arrows)) = scrolled(area, lines.len(), app.changelog_scroll);
    app.changelog_page = page;
    app.changelog_scroll = scroll;
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .block(Block::bordered().title(format!("Changelog{arrows}"))),
        area,
    );
}
