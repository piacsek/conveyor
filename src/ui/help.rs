use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::ui::style::dim;

pub const KEYS: [(&str, &str); 13] = [
    ("q", "quit"),
    ("j/k ↓/↑", "move"),
    ("h/l Tab", "focus column"),
    ("Enter/o", "open in browser"),
    ("b", "open build"),
    ("y", "copy URL"),
    ("p", "details"),
    ("z", "zoom the column"),
    ("Esc", "leave zoom"),
    ("/", "filter"),
    ("r", "refresh"),
    ("R", "refresh all"),
    ("?", "this help"),
];

pub(crate) fn draw_help(frame: &mut Frame, area: Rect) {
    let width = KEYS
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let lines: Vec<Line> = KEYS
        .iter()
        .map(|(key, what)| {
            Line::from(vec![
                Span::styled(
                    format!("{key:<width$}"),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(*what, dim()),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("Keys")),
        area,
    );
}
