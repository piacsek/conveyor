use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::Stage;
use crate::config::{Key, Keys};
use crate::ui::style::dim;

pub const KEYS: [(&str, &str); 15] = [
    ("q", "quit"),
    ("j/k ↓/↑", "move"),
    ("h/l", "focus column"),
    (TAB_KEYS, "next / previous / nth tab"),
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
pub fn keys_for(stage: Stage, keys: &Keys) -> Vec<(String, &'static str)> {
    KEYS.iter()
        .map(|(key, what)| {
            let shown = match *key {
                TAB_KEYS => format!("{} {} 1-9", label(keys.next_tab), label(keys.previous_tab)),
                key => key.to_string(),
            };
            let what = match (*key, stage) {
                ("p", Stage::Builds | Stage::Deployed) => "open the merged pull request",
                ("b", Stage::Prs) => "open the failing check, else the first",
                ("b", Stage::Queue) => "open the merge-group run",
                ("b", Stage::Builds) => "open the run",
                ("b", Stage::Deployed) => "open the main build for this sha",
                _ => what,
            };
            (shown, what)
        })
        .collect()
}

const TAB_KEYS: &str = "Tab S-Tab 1-9";

fn label(key: Key) -> String {
    match key {
        Key::Char(c) => c.to_string(),
        Key::Tab => "Tab".to_string(),
        Key::BackTab => "S-Tab".to_string(),
        Key::Ctrl(c) => format!("C-{c}"),
    }
}

pub(crate) fn draw_help(frame: &mut Frame, area: Rect, stage: Stage, configured: &Keys) {
    let keys = keys_for(stage, configured);
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
