use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, Column, Failure, Row, Stage, Write};
use crate::text::{cells, short_repo};
use crate::ui::help::{column_name, custom_title};
use crate::ui::link::link;
use crate::ui::rows::{build_row, deployed_row, pr_row, queue_row};
use crate::ui::style::dim;

type Pull = Option<(u64, String)>;

fn titled<T: Row>(base: &str, column: &Column<T>, marks: usize) -> String {
    let marks = match marks {
        0 => String::new(),
        marks => format!(" !{marks}"),
    };
    let warning = if column.error.is_some() { " ⚠" } else { "" };
    if column.is_loading() {
        format!("{base}{marks}{warning}")
    } else if column.filter.is_some() {
        format!(
            "{base} ({}/{}){marks}{warning}",
            column.visible().len(),
            column.all().len()
        )
    } else {
        format!("{base} ({}){marks}{warning}", column.all().len())
    }
}

pub(crate) fn column_title(app: &App, stage: Stage) -> String {
    let marks = app.failure_count(stage);
    match stage {
        Stage::Prs => titled(
            column_name(stage, &app.config.ui.titles),
            app.prs.column(),
            marks,
        ),
        Stage::Queue => titled(
            &named(app, stage, "Queue", app.queue.repo().map(short_repo)),
            app.queue.column(),
            marks,
        ),
        Stage::Builds => titled(
            &named(
                app,
                stage,
                "Main",
                app.builds.repo.as_deref().map(short_repo),
            ),
            &app.builds.column,
            marks,
        ),
        Stage::Deployed => titled(
            &named(app, stage, "Deployed", app.deployed.system.as_deref()),
            &app.deployed.column,
            marks,
        ),
    }
}

fn named(app: &App, stage: Stage, short: &str, suffix: Option<&str>) -> String {
    let titles = &app.config.ui.titles;
    match suffix {
        Some(suffix) => format!("{} {suffix}", custom_title(stage, titles).unwrap_or(short)),
        None => column_name(stage, titles).to_string(),
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
    let view = app.stage(stage);
    let (area, block) = match tab_bar(view.tab_names(), view.active_tab(), area.width) {
        Some(bar) => {
            frame.render_widget(block, area);
            let line = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1)
                .intersection(area);
            frame.render_widget(Paragraph::new(bar), line);
            let below = Rect::new(
                area.x,
                area.y.saturating_add(2),
                area.width,
                area.height.saturating_sub(2),
            )
            .intersection(area);
            let sides = Block::bordered().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM);
            (below, sides)
        }
        None => (area, block),
    };
    let focused = stage == app.focus;
    let now = app.now;
    let jobs = std::mem::take(&mut app.jobs);
    let repo = app.builds.repo.clone();
    let no_builds = format!("no builds on {}", app.builds.branch);
    let linked = !app.demo;
    let icons = app.config.ui.icons;
    match stage {
        Stage::Prs => {
            let spinner = app.spinner();
            let busy: Vec<Option<String>> = app
                .prs
                .column()
                .visible()
                .iter()
                .map(|pr| app.write_on(Stage::Prs, pr.key()).map(Write::word))
                .collect();
            let failures = marks(app, Stage::Prs, app.prs.column());
            draw_list(
                frame,
                app.prs.column_mut(),
                area,
                block,
                focused,
                "no open pull requests",
                |i, pr, on, w| {
                    let word = busy.get(i).cloned().flatten();
                    let busy = word.as_deref().map(|word| (spinner, word));
                    let pull = linked.then(|| (pr.number, pr.url.clone()));
                    let failure = failures.get(i).and_then(Option::as_ref);
                    (pr_row(pr, icons, busy, failure, now, on, w), pull)
                },
            )
        }
        Stage::Queue => draw_list(
            frame,
            app.queue.column_mut(),
            area,
            block,
            focused,
            "queue empty",
            |_, entry, on, w| {
                let jobs = entry.run.as_ref().and_then(|run| jobs.get(&run.id));
                let pull = linked.then(|| (entry.number, entry.url.clone()));
                (queue_row(entry, jobs, now, on, w), pull)
            },
        ),
        Stage::Builds => {
            let failures = marks(app, Stage::Builds, &app.builds.column);
            draw_list(
                frame,
                &mut app.builds.column,
                area,
                block,
                focused,
                &no_builds,
                |i, build, on, w| {
                    let failure = failures.get(i).and_then(Option::as_ref);
                    let pull = build.pull_request(repo.as_deref()).filter(|_| linked);
                    (
                        build_row(build, jobs.get(&build.id), failure, now, on, w),
                        pull,
                    )
                },
            )
        }
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
                        .map(|pull| (pull.number, pull.url.clone()))
                        .filter(|_| linked);
                    let behind = behind.get(i).copied().flatten();
                    (
                        deployed_row(row, behind, &app.builds.branch, now, on, w),
                        pull,
                    )
                },
            )
        }
    }
    app.jobs = jobs;
}

fn tab_bar(names: &[String], active: usize, width: u16) -> Option<Line<'static>> {
    if names.len() < 2 {
        return None;
    }
    let width = usize::from(width.saturating_sub(2));
    const SEPARATOR: &str = " │ ";
    let gap = SEPARATOR.chars().count();
    let shown = |start: usize| {
        let lead = if start > 0 { 1 + gap } else { 0 };
        lead + names[start..=active]
            .iter()
            .map(|name| name.chars().count())
            .sum::<usize>()
            + (active - start) * gap
    };
    let start = (0..active)
        .find(|&start| shown(start) < width)
        .unwrap_or(active);
    let mut parts: Vec<(String, Style)> = Vec::new();
    if start > 0 {
        parts.push(("…".to_string(), dim()));
        parts.push((SEPARATOR.to_string(), Style::default()));
    }
    for (i, name) in names.iter().enumerate().skip(start) {
        if i > start {
            parts.push((SEPARATOR.to_string(), Style::default()));
        }
        let style = if i == active {
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            dim()
        };
        parts.push((name.clone(), style));
    }
    let total: usize = parts.iter().map(|(text, _)| text.chars().count()).sum();
    let mut used = 0;
    let mut spans = Vec::new();
    for (text, style) in parts {
        let count = text.chars().count();
        if total <= width || used + count < width {
            used += count;
            spans.push(Span::styled(text, style));
        } else {
            let cut: String = text.chars().take(width.saturating_sub(used + 1)).collect();
            spans.push(Span::styled(format!("{cut}…"), style));
            break;
        }
    }
    Some(Line::from(spans))
}

fn marks<T: Row>(app: &App, stage: Stage, column: &Column<T>) -> Vec<Option<Failure>> {
    column
        .visible()
        .iter()
        .map(|row| app.failure_on(stage, row.key()).cloned())
        .collect()
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
            .as_ref()
            .map_or_else(|| "fetching…".to_string(), ToString::to_string);
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
    let heights: Vec<u16> = cards.iter().map(|card| cells(card.len())).collect();
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
