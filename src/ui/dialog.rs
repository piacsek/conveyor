use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Padding, Paragraph};

use crate::app::{Menu, Pending};
use crate::config::UpdateMethod;
use crate::model::prs::StatusChange;
use crate::text::wrap;
use crate::ui::style::dim;

const HINT: &str = "y confirms · any other key cancels";
const MAX_INNER: usize = 56;
const MAX_LINES: usize = 3;

fn question(pending: &Pending) -> String {
    match pending {
        Pending::Merge(target) => format!("merge #{} {}", target.number, target.title),
        Pending::UpdateBranch(target) => {
            let verb = match target.method {
                UpdateMethod::Merge => "update",
                UpdateMethod::Rebase => "rebase",
            };
            format!("{verb} branch of #{} {}", target.number, target.title)
        }
        Pending::Status(target) => match target.change {
            StatusChange::EnableAutoMerge { .. } => {
                format!("enable auto-merge for #{} {}", target.number, target.title)
            }
            _ => format!("close #{} {}", target.number, target.title),
        },
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

const OFFER_LINES: usize = 2;

pub(crate) fn draw_manage(frame: &mut Frame, area: Rect, menu: &Menu) {
    let room = usize::from(area.width.saturating_sub(8)).min(MAX_INNER);
    let offers: Vec<(String, bool)> = menu
        .offers
        .iter()
        .flat_map(|offer| {
            let text = match &offer.choice {
                Ok(_) => offer.label.clone(),
                Err(reason) => format!("{} · {reason}", offer.label),
            };
            let live = offer.choice.is_ok();
            wrap(&text, room.saturating_sub(3), OFFER_LINES)
                .into_iter()
                .enumerate()
                .map(move |(i, line)| {
                    let lead = if i == 0 {
                        format!("{}  ", offer.key)
                    } else {
                        "   ".to_string()
                    };
                    (format!("{lead}{line}"), live)
                })
        })
        .collect();
    let inner = offers
        .iter()
        .map(|(line, _)| line.chars().count())
        .chain([menu.title.chars().count()])
        .max()
        .unwrap_or(0)
        .min(room);
    let height = offers.len() + 2;
    let text: Vec<Line> = offers
        .into_iter()
        .map(|(line, live)| {
            if live {
                Line::from(line)
            } else {
                Line::from(Span::styled(line, dim()))
            }
        })
        .collect();
    let dialog = area.centered(
        Constraint::Length(inner as u16 + 4),
        Constraint::Length(height as u16),
    );
    frame.render_widget(Clear, dialog);
    frame.render_widget(
        Paragraph::new(text).block(
            Block::bordered()
                .title(menu.title.clone())
                .padding(Padding::horizontal(1)),
        ),
        dialog,
    );
}
