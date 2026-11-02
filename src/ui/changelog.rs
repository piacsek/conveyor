use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::text::wrap;
use crate::ui::details::scrolled;

fn hanging(text: &str, first: &str, rest: &str, width: usize) -> Vec<String> {
    let indent = first.chars().count();
    match wrap(text, width.saturating_sub(indent), usize::MAX).as_slice() {
        [] => vec![first.trim_end().to_string()],
        wrapped => wrapped
            .iter()
            .enumerate()
            .map(|(i, part)| format!("{}{part}", if i == 0 { first } else { rest }))
            .collect(),
    }
}

pub fn lines(markdown: &str, width: usize) -> Vec<Line<'static>> {
    if let Some(heading) = markdown
        .strip_prefix('#')
        .map(|rest| rest.trim_start_matches('#'))
        .and_then(|rest| rest.strip_prefix(' '))
    {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        return hanging(heading, "", "", width)
            .into_iter()
            .map(|part| Line::styled(part, bold))
            .collect();
    }
    let wrapped = match markdown.strip_prefix("- ") {
        Some(bullet) => hanging(bullet, "- ", "  ", width),
        None if markdown.starts_with(char::is_whitespace) => hanging(markdown, "  ", "  ", width),
        None => hanging(markdown, "", "", width),
    };
    wrapped.into_iter().map(Line::raw).collect()
}

pub(crate) fn draw_changelog(frame: &mut Frame, app: &mut App, area: Rect) {
    let width = usize::from(area.width.saturating_sub(2));
    let lines: Vec<Line> = app
        .changelog
        .lines()
        .flat_map(|markdown| lines(markdown, width))
        .collect();
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
