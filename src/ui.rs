use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, List, ListItem, Paragraph};

use crate::app::{App, ColumnState};
use crate::model::prs::{CheckState, PullRequest};
use crate::text::{age, pad_right};

const HIGHLIGHT: &str = "> ";

pub fn draw(frame: &mut Frame, app: &mut App) {
    let columns = Layout::horizontal([Constraint::Fill(1); 4]).split(frame.area());
    draw_prs(frame, app, columns[0]);
    for (area, name) in columns[1..]
        .iter()
        .zip(["Merge queue", "Main builds", "Deployed"])
    {
        frame.render_widget(
            Paragraph::new("not configured").block(Block::bordered().title(name)),
            *area,
        );
    }
}

fn draw_prs(frame: &mut Frame, app: &mut App, area: Rect) {
    match &app.prs {
        ColumnState::Loading => {
            frame.render_widget(
                Paragraph::new("fetching…").block(Block::bordered().title("My PRs")),
                area,
            );
        }
        ColumnState::Ready(prs) if prs.is_empty() => {
            frame.render_widget(
                Paragraph::new("no open pull requests")
                    .block(Block::bordered().title("My PRs (0)")),
                area,
            );
        }
        ColumnState::Ready(prs) => {
            let width = usize::from(area.width.saturating_sub(2 + HIGHLIGHT.len() as u16));
            let items: Vec<ListItem> = prs
                .iter()
                .enumerate()
                .map(|(i, pr)| ListItem::new(row(i, pr, app.now, width)))
                .collect();
            let list = List::new(items)
                .block(Block::bordered().title(format!("My PRs ({})", prs.len())))
                .highlight_symbol(HIGHLIGHT);
            frame.render_stateful_widget(list, area, &mut app.list);
        }
    }
}

fn row(index: usize, pr: &PullRequest, now: std::time::SystemTime, width: usize) -> String {
    let number = if index < 9 {
        (index + 1).to_string()
    } else {
        " ".to_string()
    };
    let repo = pr.repo.rsplit('/').next().unwrap_or(&pr.repo);
    let left = format!(
        "{number} {} #{} {repo}  {}",
        glyph(pr.checks),
        pr.number,
        pr.title
    );
    let right = pr.updated_at.map(|at| age(at, now)).unwrap_or_default();
    let left_width = width.saturating_sub(right.chars().count() + 1);
    format!("{} {right}", pad_right(&left, left_width))
}

fn glyph(checks: CheckState) -> char {
    match checks {
        CheckState::Success => '✓',
        CheckState::Failure => '✗',
        CheckState::Pending => '●',
        CheckState::None => '○',
        CheckState::Unknown => '?',
    }
}
