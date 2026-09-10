use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, List, ListItem, Paragraph};

use crate::app::{App, ColumnState};
use crate::model::prs::{CheckConclusion, CheckState, PullRequest};
use crate::text::{age, pad_right};

const HIGHLIGHT: &str = "> ";

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, footer] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    let body = if app.details {
        let [columns, details] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(40)]).areas(body);
        draw_details(frame, app, details);
        columns
    } else {
        body
    };
    let columns = Layout::horizontal([Constraint::Fill(1); 4]).split(body);
    frame.render_widget(Paragraph::new(footer_text(app)), footer);
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
        ColumnState::Ready { rows: prs, .. } if prs.is_empty() => {
            frame.render_widget(
                Paragraph::new("no open pull requests")
                    .block(Block::bordered().title("My PRs (0)")),
                area,
            );
        }
        ColumnState::Ready { rows: prs, .. } => {
            let title = format!("My PRs ({})", prs.len());
            let visible = app.visible();
            if visible.is_empty() {
                let query = app.filter().unwrap_or_default();
                frame.render_widget(
                    Paragraph::new(format!("no matches for /{query}"))
                        .block(Block::bordered().title(title)),
                    area,
                );
                return;
            }
            let width = usize::from(area.width.saturating_sub(2 + HIGHLIGHT.len() as u16));
            let items: Vec<ListItem> = visible
                .iter()
                .enumerate()
                .map(|(i, pr)| ListItem::new(row(i, pr, app.now, width)))
                .collect();
            let list = List::new(items)
                .block(Block::bordered().title(title))
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

fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let Some(pr) = app.selected() else {
        frame.render_widget(
            Paragraph::new("nothing selected").block(Block::bordered().title("Details")),
            area,
        );
        return;
    };
    let mut lines = vec![
        Line::from(format!(
            "#{} {}  {}  +{} −{}",
            pr.number, pr.repo, pr.head_ref, pr.additions, pr.deletions
        )),
        Line::from(format!("review: {}  merge: {}", pr.review, pr.merge_state)),
    ];
    lines.extend(pr.checks_failures_first().into_iter().map(|check| {
        Line::from(format!(
            "{} {}  {}",
            conclusion_glyph(check.conclusion),
            check.name,
            check.url
        ))
    }));
    let title = format!("#{} {}", pr.number, pr.title);
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title)),
        area,
    );
}

fn conclusion_glyph(conclusion: CheckConclusion) -> char {
    match conclusion {
        CheckConclusion::Success => '✓',
        CheckConclusion::Failure => '✗',
        CheckConclusion::Pending => '●',
        CheckConclusion::Skipped => '-',
        CheckConclusion::Unknown => '?',
    }
}

fn footer_text(app: &App) -> String {
    if let Some(notice) = &app.notice {
        return notice.clone();
    }
    match (app.filter(), app.fetched_at()) {
        (Some(query), _) => format!("/{query}  {}/{}", app.visible().len(), app.all().len()),
        (None, Some(at)) => format!("refreshed {} ago", age(at, app.now)),
        (None, None) => String::new(),
    }
}
