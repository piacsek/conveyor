use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph};

use crate::app::{App, Column, Failure, Row, Stage, Write};
use crate::ui::rows::{build_row, deployed_row, pr_row, queue_row};
use crate::ui::style::{dim, short_repo};

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
    match stage {
        Stage::Prs => {
            let spinner = app.spinner();
            let busy: Vec<Option<String>> = app
                .prs
                .column
                .visible()
                .iter()
                .map(|pr| app.write_on(Stage::Prs, pr.key()).map(Write::word))
                .collect();
            let failures: Vec<Option<Failure>> = app
                .prs
                .column
                .visible()
                .iter()
                .map(|pr| app.failure_on(Stage::Prs, pr.key()).cloned())
                .collect();
            draw_list(
                frame,
                &mut app.prs.column,
                area,
                block,
                focused,
                "no open pull requests",
                |i, pr, on, w| {
                    let word = busy.get(i).cloned().flatten();
                    let busy = word.as_deref().map(|word| (spinner, word));
                    pr_row(
                        pr,
                        busy,
                        failures.get(i).and_then(Option::as_ref),
                        now,
                        on,
                        w,
                    )
                },
            )
        }
        Stage::Queue => draw_list(
            frame,
            &mut app.queue.column,
            area,
            block,
            focused,
            "queue empty",
            |_, entry, on, w| {
                let jobs = entry.run.as_ref().and_then(|run| jobs.get(&run.id));
                queue_row(entry, jobs, now, on, w)
            },
        ),
        Stage::Builds => draw_list(
            frame,
            &mut app.builds.column,
            area,
            block,
            focused,
            "no builds on main",
            |_, build, on, w| build_row(build, jobs.get(&build.id), now, on, w),
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
                |i, row, on, w| deployed_row(row, behind.get(i).copied().flatten(), now, on, w),
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
    row: impl Fn(usize, &T, bool, usize) -> Vec<Line<'static>>,
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
    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, item)| ListItem::new(Text::from(row(i, item, Some(i) == selected, width))))
        .collect();
    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut column.list);
}
