use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Padding, Paragraph};

use crate::app::Pending;
use crate::config::UpdateMethod;
use crate::text::wrap;
use crate::ui::style::dim;

const HINT: &str = "y confirms · any other key cancels";
const MAX_INNER: usize = 56;
const MAX_LINES: usize = 3;

fn question(pending: &Pending) -> String {
    match pending {
        Pending::Rerun(rerun) => format!("rerun failed jobs of {}", rerun.label),
        Pending::Merge(target) => format!("merge #{} {}", target.number, target.title),
        Pending::UpdateBranch(target) => {
            let verb = match target.method {
                UpdateMethod::Merge => "update",
                UpdateMethod::Rebase => "rebase",
            };
            format!("{verb} branch of #{} {}", target.number, target.title)
        }
    }
}

pub(crate) fn draw_confirm(frame: &mut Frame, area: Rect, pending: &Pending) {
    let room = usize::from(area.width.saturating_sub(8)).min(MAX_INNER);
    let mut lines = wrap(&question(pending), room.saturating_sub(1), MAX_LINES);
    if let Some(last) = lines.last_mut() {
        last.push('?');
    }
    let hint = wrap(HINT, room, 2);
    let inner = lines
        .iter()
        .chain(&hint)
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .min(room);
    let height = lines.len() + hint.len() + 3;
    let mut text: Vec<Line> = lines.into_iter().map(Line::from).collect();
    text.push(Line::default());
    text.extend(
        hint.into_iter()
            .map(|line| Line::from(Span::styled(line, dim()))),
    );
    let dialog = area.centered(
        Constraint::Length(inner as u16 + 4),
        Constraint::Length(height as u16),
    );
    frame.render_widget(Clear, dialog);
    frame.render_widget(
        Paragraph::new(text).block(
            Block::bordered()
                .title("Confirm")
                .padding(Padding::horizontal(1)),
        ),
        dialog,
    );
}
