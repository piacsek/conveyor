use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph};

use crate::app::{App, Column, Row, Stage};
use crate::ui::link::link;
use crate::ui::rows::{build_row, deployed_row, pr_row, queue_row};
use crate::ui::style::{dim, short_repo};

type Pull = Option<(u64, String)>;

fn titled<T: Row>(base: &str, column: &Column<T>) -> String {
    let warning = if column.error.is_some() { " ⚠" } else { "" };
    if column.is_loading() {
        format!("{base}{warning}")
    } else if column.filter.is_some() {
        format!(
            "{base} ({}/{}){warning}",
            column.visible().len(),
            column.all().len()
        )
    } else {
        format!("{base} ({}){warning}", column.all().len())
    }
}

pub(crate) fn column_title(app: &App, stage: Stage) -> String {
    match stage {
        Stage::Prs => titled("My PRs", &app.prs.column),
        Stage::Queue => match &app.queue.repo {
            Some(repo) => titled(&format!("Queue {}", short_repo(repo)), &app.queue.column),
            None => titled("Merge queue", &app.queue.column),
        },
        Stage::Builds => match &app.builds.repo {
            Some(repo) => titled(&format!("Main {}", short_repo(repo)), &app.builds.column),
            None => titled("Main builds", &app.builds.column),
        },
        Stage::Deployed => match &app.deployed.system {
            Some(system) => titled(&format!("Deployed {system}"), &app.deployed.column),
            None => titled("Deployed", &app.deployed.column),
        },
    }
}

fn column_block(app: &App, stage: Stage, titled: bool) -> Block<'static> {
    if !titled {
        return Block::bordered();
    }
    let style = if stage == app.focus {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        dim()
    };
    Block::bordered().title(Span::styled(column_title(app, stage), style))
}

pub(crate) fn draw_column(
    frame: &mut Frame,
    app: &mut App,
    stage: Stage,
    area: Rect,
    titled: bool,
) {
    let block = column_block(app, stage, titled);
    let focused = stage == app.focus;
    let now = app.now;
    let jobs = std::mem::take(&mut app.jobs);
    let repo = app.builds.repo.clone();
    match stage {
        Stage::Prs => draw_list(
            frame,
            &mut app.prs.column,
            area,
            block,
            focused,
            "no open pull requests",
            |_, pr, on, w| {
                let pull = Some((pr.number, pr.url.clone()));
                (pr_row(pr, now, on, w), pull)
            },
        ),
        Stage::Queue => draw_list(
            frame,
            &mut app.queue.column,
            area,
            block,
            focused,
            "queue empty",
            |_, entry, on, w| {
                let jobs = entry.run.as_ref().and_then(|run| jobs.get(&run.id));
                let pull = Some((entry.number, entry.url.clone()));
                (queue_row(entry, jobs, now, on, w), pull)
            },
        ),
        Stage::Builds => draw_list(
            frame,
            &mut app.builds.column,
            area,
            block,
            focused,
            "no builds on main",
            |_, build, on, w| {
                let pull = build.pull_request(repo.as_deref());
                (build_row(build, jobs.get(&build.id), now, on, w), pull)
            },
        ),
        Stage::Deployed
            if app.deployed.column.is_loading()
                && app.deployed.column.error.is_none()
                && !app.deploy_configured() =>
        {
            frame.render_widget(
                Paragraph::new("no [[repo.deploy]] configured").block(block),
                area,
            )
        }
        Stage::Deployed => {
            let behind: Vec<Option<usize>> = app
                .deployed
                .column
                .visible()
                .iter()
                .map(|row| app.behind_main(row))
                .collect();
            draw_list(
                frame,
                &mut app.deployed.column,
                area,
                block,
                focused,
                "no environments",
                |i, row, on, w| {
                    let pull = row
                        .pull
                        .as_ref()
                        .map(|pull| (pull.number, pull.url.clone()));
                    let behind = behind.get(i).copied().flatten();
                    (deployed_row(row, behind, now, on, w), pull)
                },
            )
        }
    }
    app.jobs = jobs;
}

fn draw_list<T: Row>(
    frame: &mut Frame,
    column: &mut Column<T>,
    area: Rect,
    block: Block<'static>,
    focused: bool,
    empty: &str,
    row: impl Fn(usize, &T, bool, usize) -> (Vec<Line<'static>>, Pull),
) {
    if column.is_loading() {
        let body = column
            .error
            .clone()
            .unwrap_or_else(|| "fetching…".to_string());
        frame.render_widget(Paragraph::new(body).block(block), area);
        return;
    }
    if column.all().is_empty() {
        frame.render_widget(Paragraph::new(empty.to_string()).block(block), area);
        return;
    }
    let visible = column.visible();
    if visible.is_empty() {
        let query = column.filter.clone().unwrap_or_default();
        frame.render_widget(
            Paragraph::new(format!("no matches for /{query}")).block(block),
            area,
        );
        return;
    }
    let width = usize::from(area.width.saturating_sub(2));
    let selected = column.list.selected().filter(|_| focused);
    let (cards, pulls): (Vec<Vec<Line<'static>>>, Vec<Pull>) = visible
        .iter()
        .enumerate()
        .map(|(i, item)| row(i, item, Some(i) == selected, width))
        .unzip();
    let heights: Vec<u16> = cards.iter().map(|card| card.len() as u16).collect();
    let inner = block.inner(area);
    let items: Vec<ListItem> = cards
        .into_iter()
        .map(|card| ListItem::new(Text::from(card)))
        .collect();
    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut column.list);
    let mut top = inner.y;
    for (height, pull) in heights.into_iter().zip(pulls).skip(column.list.offset()) {
        if top >= inner.bottom() {
            break;
        }
        if let Some((number, url)) = pull {
            let card = Rect::new(inner.x, top, inner.width, height).intersection(inner);
            link(frame.buffer_mut(), card, number, &url);
        }
        top = top.saturating_add(height);
    }
}
