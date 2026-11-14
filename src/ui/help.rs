use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::Stage;
use crate::config::ViewKey;
use crate::ui::style::dim;

pub const KEYS: [(&str, &str); 15] = [
    ("q", "quit"),
    ("j/k ↓/↑", "move"),
    ("h/l", "focus column"),
    ("Tab 1-9", "switch list"),
    ("p", "open pull request"),
    ("b", "open build"),
    ("y/Y", "copy pull request / build URL"),
    ("d C-d/C-u", "details / scroll them"),
    ("L", "log of the failed step"),
    ("S", "manage: merge, update, rerun, status, checkout"),
    ("z", "zoom the column"),
    ("Esc", "close details / zoom / mark / filter"),
    ("/", "filter (Enter keeps, Esc clears)"),
    ("r/R", "refresh focused / all"),
    ("?/c", "help / changelog"),
];

pub fn column_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Prs => "My PRs",
        Stage::Queue => "Merge queue",
        Stage::Builds => "Main builds",
        Stage::Deployed => "Deployed",
    }
}

/// `KEYS` with `p` and `b` saying what they do in the focused column: the keys are the same
/// everywhere, their target is not.
pub fn keys_for(stage: Stage, next_view: ViewKey) -> Vec<(String, &'static str)> {
    KEYS.iter()
        .map(|(key, what)| {
            let key = match (*key, next_view) {
                ("Tab 1-9", ViewKey::Char(c)) => format!("{c} 1-9"),
                _ => key.to_string(),
            };
            let what = match (key.as_str(), stage) {
                ("p", Stage::Builds | Stage::Deployed) => "open the merged pull request",
                ("b", Stage::Prs) => "open the failing check, else the first",
                ("b", Stage::Queue) => "open the merge-group run",
                ("b", Stage::Builds) => "open the run",
                ("b", Stage::Deployed) => "open the main build for this sha",
                _ => what,
            };
            (key, what)
        })
        .collect()
}

pub(crate) fn draw_help(frame: &mut Frame, area: Rect, stage: Stage, next_view: ViewKey) {
    let keys = keys_for(stage, next_view);
    let width = keys
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let lines: Vec<Line> = keys
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
        Paragraph::new(lines)
            .block(Block::bordered().title(format!("Keys \u{2014} {}", column_name(stage)))),
        area,
    );
}
