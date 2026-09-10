use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, Paragraph};

use crate::app::{App, ColumnState, Mode, Stage};
use crate::model::prs::{CheckConclusion, CheckState, PullRequest};
use crate::text::{age, pad_right};

const HIGHLIGHT: &str = "> ";

pub const KEYS: [(&str, &str); 10] = [
    ("j/k ↓/↑", "move"),
    ("1-9", "jump to row"),
    ("h/l Tab", "focus column"),
    ("Enter/o", "open in browser"),
    ("y", "copy URL"),
    ("p", "details"),
    ("/", "filter"),
    ("r", "refresh"),
    ("?", "this help"),
    ("q Esc", "quit"),
];

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, footer] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    if app.mode == Mode::Help {
        draw_help(frame, body);
        frame.render_widget(Paragraph::new("any key returns"), footer);
        return;
    }
    let body = if app.details {
        let details_percent = app.config.ui.details_percent;
        let [columns, details] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(details_percent)])
                .areas(body);
        draw_details(frame, app, details);
        columns
    } else {
        body
    };
    frame.render_widget(Paragraph::new(footer_text(app)), footer);
    if body.width < app.config.ui.min_column_width * Stage::ALL.len() as u16 {
        draw_tabs(frame, app, body);
    } else {
        let columns = Layout::horizontal([Constraint::Fill(1); 4]).split(body);
        for (stage, area) in Stage::ALL.into_iter().zip(columns.iter()) {
            draw_column(frame, app, stage, *area);
        }
    }
}

fn draw_tabs(frame: &mut Frame, app: &mut App, area: Rect) {
    let [bar, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
    let mut spans = Vec::new();
    for (i, stage) in Stage::ALL.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" │ "));
        }
        let style = if stage == app.focus {
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        spans.push(Span::styled(column_title(app, stage), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), bar);
    draw_column(frame, app, app.focus, body);
}

fn column_title(app: &App, stage: Stage) -> String {
    match stage {
        Stage::Prs => {
            let warning = if app.prs_error.is_some() { " ⚠" } else { "" };
            match &app.prs {
                ColumnState::Loading => format!("My PRs{warning}"),
                ColumnState::Ready { rows, .. } => format!("My PRs ({}){warning}", rows.len()),
            }
        }
        Stage::Queue => "Merge queue".to_string(),
        Stage::Builds => "Main builds".to_string(),
        Stage::Deployed => "Deployed".to_string(),
    }
}

fn column_block(app: &App, stage: Stage) -> Block<'static> {
    let style = if stage == app.focus {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    Block::bordered().title(Span::styled(column_title(app, stage), style))
}

fn draw_column(frame: &mut Frame, app: &mut App, stage: Stage, area: Rect) {
    match stage {
        Stage::Prs => draw_prs(frame, app, area),
        _ => frame.render_widget(
            Paragraph::new("not configured").block(column_block(app, stage)),
            area,
        ),
    }
}

fn draw_prs(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = column_block(app, Stage::Prs);
    match &app.prs {
        ColumnState::Loading => {
            let body = app
                .prs_error
                .clone()
                .unwrap_or_else(|| "fetching…".to_string());
            frame.render_widget(Paragraph::new(body).block(block), area);
        }
        ColumnState::Ready { rows: prs, .. } if prs.is_empty() => {
            frame.render_widget(Paragraph::new("no open pull requests").block(block), area);
        }
        ColumnState::Ready { .. } => {
            let visible = app.visible();
            if visible.is_empty() {
                let query = app.filter().unwrap_or_default();
                frame.render_widget(
                    Paragraph::new(format!("no matches for /{query}")).block(block),
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
            let list = List::new(items).block(block).highlight_symbol(HIGHLIGHT);
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
    if let Some(error) = &app.prs_error {
        return error.clone();
    }
    match (app.filter(), app.fetched_at()) {
        (Some(query), _) => format!("/{query}  {}/{}", app.visible().len(), app.all().len()),
        (None, Some(at)) => format!("refreshed {} ago", age(at, app.now)),
        (None, None) => String::new(),
    }
}

fn draw_help(frame: &mut Frame, area: Rect) {
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
                Span::styled(*what, Style::default().add_modifier(Modifier::DIM)),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("Keys")),
        area,
    );
}
