#![allow(dead_code)]

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

use conveyor::app::{
    App, CheckoutTarget, Input, MergeTarget, Request, Rows, Stage, StatusTarget, UpdateTarget, run,
};
use conveyor::config::{Config, PrsView, UpdateMethod};
use conveyor::model::builds::{Build, BuildStatus, Builds, PullRef};
use conveyor::model::deployed::{Condition, Deployed, Deployment, Waiting};
use conveyor::model::jobs::Job;
use conveyor::model::prs::{CheckState, PullRequest, StatusChange};
use conveyor::model::queue::{Queue, QueueEntry, QueueState};
use conveyor::open::Opener;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub const NOW: u64 = 1_800_000_000;

#[derive(Default)]
pub struct FakeOpener {
    opened: RefCell<Vec<String>>,
    copied: RefCell<Vec<String>>,
    failure: RefCell<Option<String>>,
}

impl FakeOpener {
    pub fn opened(&self) -> Vec<String> {
        self.opened.borrow().clone()
    }

    pub fn copied(&self) -> Vec<String> {
        self.copied.borrow().clone()
    }

    pub fn fail_next(&self, message: &str) {
        *self.failure.borrow_mut() = Some(message.to_string());
    }

    fn maybe_fail(&self) -> io::Result<()> {
        match self.failure.borrow_mut().take() {
            Some(message) => Err(io::Error::other(message)),
            None => Ok(()),
        }
    }
}

impl Opener for FakeOpener {
    fn open(&self, url: &str) -> io::Result<()> {
        self.maybe_fail()?;
        self.opened.borrow_mut().push(url.to_string());
        Ok(())
    }

    fn copy(&self, text: &str) -> io::Result<()> {
        self.maybe_fail()?;
        self.copied.borrow_mut().push(text.to_string());
        Ok(())
    }
}

pub fn key(code: KeyCode) -> io::Result<Input> {
    Ok(Input::Key(KeyEvent::new(code, KeyModifiers::NONE)))
}

pub fn ctrl(c: char) -> io::Result<Input> {
    Ok(Input::Key(KeyEvent::new(
        KeyCode::Char(c),
        KeyModifiers::CONTROL,
    )))
}

pub fn prs(prs: Vec<PullRequest>) -> io::Result<Input> {
    prs_in(0, prs)
}

pub fn prs_in(view: usize, prs: Vec<PullRequest>) -> io::Result<Input> {
    Ok(Input::Data(Stage::Prs, view, Ok(Rows::Prs(prs))))
}

pub fn two_views() -> Config {
    let mut config = Config::default();
    config.prs.view.push(PrsView {
        name: "Review".to_string(),
        query: "is:open review-requested:@me".to_string(),
        limit: None,
    });
    config
}

pub fn queue(entries: Vec<QueueEntry>) -> io::Result<Input> {
    Ok(Input::Data(
        Stage::Queue,
        0,
        Ok(Rows::Queue(Queue {
            repo: "acme/webapp".to_string(),
            url: "https://github.com/acme/webapp/queue/main".to_string(),
            entries,
        })),
    ))
}

pub fn builds(builds: Vec<Build>) -> io::Result<Input> {
    Ok(Input::Data(
        Stage::Builds,
        0,
        Ok(Rows::Builds(Builds {
            repo: "acme/webapp".to_string(),
            builds,
        })),
    ))
}

pub fn build(run_number: u64, status: BuildStatus, pr: Option<(u64, &str, &str)>) -> Build {
    let started = UNIX_EPOCH + Duration::from_secs(NOW - 2 * 3600);
    Build {
        id: 1000 + run_number,
        run_number,
        attempt: 1,
        status,
        sha: format!("{run_number:040x}"),
        title: pr
            .map(|(_, _, title)| title.to_string())
            .unwrap_or_else(|| format!("run {run_number}")),
        actor: "bot".to_string(),
        url: format!(
            "https://github.com/acme/webapp/actions/runs/{}",
            1000 + run_number
        ),
        started_at: Some(started),
        finished_at: Some(started + Duration::from_secs(228)),
        pr_number: pr.map(|(number, _, _)| number),
        pull: pr.map(pull_ref),
    }
}

pub fn jobs(run_id: u64, jobs: Vec<Job>) -> io::Result<Input> {
    Ok(Input::Jobs(run_id, Ok(jobs)))
}

pub fn log(job_id: u64, lines: Vec<&str>) -> io::Result<Input> {
    Ok(Input::Log(
        job_id,
        Ok(lines.into_iter().map(str::to_string).collect()),
    ))
}

pub fn rerun_done(results: Vec<(u64, Result<(), &str>)>) -> io::Result<Input> {
    Ok(Input::Rerun(
        results
            .into_iter()
            .map(|(run_id, result)| (run_id, result.map_err(str::to_string)))
            .collect(),
    ))
}

pub fn jobs_failed(run_id: u64, message: &str) -> io::Result<Input> {
    Ok(Input::Jobs(run_id, Err(message.to_string())))
}

pub fn job(id: u64, name: &str, status: BuildStatus, failed_step: Option<&str>) -> Job {
    let started = UNIX_EPOCH + Duration::from_secs(NOW - 3600);
    Job {
        id,
        name: name.to_string(),
        attempt: 1,
        status,
        started_at: Some(started),
        completed_at: Some(started + Duration::from_secs(19)),
        url: format!("https://github.com/acme/webapp/actions/runs/1025/job/{id}"),
        failed_step: failed_step.map(str::to_string),
    }
}

pub fn deployed(rows: Vec<Deployment>) -> io::Result<Input> {
    Ok(Input::Data(
        Stage::Deployed,
        0,
        Ok(Rows::Deployed(Deployed {
            system: "api".to_string(),
            rows,
        })),
    ))
}

pub fn deployment(env: &str, sha_seed: u64, pr: Option<(u64, &str, &str)>) -> Deployment {
    Deployment {
        env: env.to_string(),
        image: Some(format!("ghcr.io/acme/api:{sha_seed:040x}")),
        sha: Some(format!("{sha_seed:040x}")),
        pull: pr.map(pull_ref),
        error: None,
        fetched_at: Some(UNIX_EPOCH + Duration::from_secs(NOW - 6 * 86_400)),
        ..Deployment::default()
    }
}

pub fn condition(kind: &str, status: &str, reason: &str, message: &str) -> Condition {
    Condition {
        kind: kind.to_string(),
        status: status.to_string(),
        reason: reason.to_string(),
        message: message.to_string(),
    }
}

pub fn waiting(reason: &str) -> Waiting {
    Waiting {
        reason: reason.to_string(),
        message: String::new(),
    }
}

pub fn entry(position: u64, number: u64, author: &str, title: &str) -> QueueEntry {
    QueueEntry {
        position,
        state: QueueState::Queued,
        number,
        title: title.to_string(),
        author: author.to_string(),
        url: format!("https://github.com/acme/webapp/pull/{number}"),
        head_sha: format!("{number:040x}"),
        checks: CheckState::None,
        eta: None,
        enqueued_at: Some(UNIX_EPOCH + Duration::from_secs(NOW - 20 * 60)),
        solo: false,
        jump: false,
        run: None,
    }
}

pub fn failed(stage: Stage, message: &str) -> io::Result<Input> {
    failed_in(stage, 0, message)
}

pub fn failed_in(stage: Stage, view: usize, message: &str) -> io::Result<Input> {
    Ok(Input::Data(stage, view, Err(message.to_string())))
}

pub fn fetching(stage: Stage) -> io::Result<Input> {
    fetching_in(stage, 0)
}

pub fn fetching_in(stage: Stage, view: usize) -> io::Result<Input> {
    Ok(Input::Fetching(stage, view))
}

pub fn tick_at_ms(millis: u64) -> io::Result<Input> {
    Ok(Input::Tick(UNIX_EPOCH + Duration::from_millis(millis)))
}

pub fn tick_at(secs: u64) -> io::Result<Input> {
    Ok(Input::Tick(UNIX_EPOCH + Duration::from_secs(secs)))
}

pub struct Harness {
    pub terminal: Terminal<TestBackend>,
    pub app: App,
    pub opener: FakeOpener,
    pub requests: Vec<Request>,
}

impl Harness {
    pub fn new() -> Self {
        Self::with_size(160, 12)
    }

    pub fn with_size(width: u16, height: u16) -> Self {
        Self::with_config(width, height, Config::default())
    }

    pub fn with_config(width: u16, height: u16, config: Config) -> Self {
        Self {
            terminal: Terminal::new(TestBackend::new(width, height)).unwrap(),
            app: App::at(UNIX_EPOCH + Duration::from_secs(NOW), config),
            opener: FakeOpener::default(),
            requests: Vec::new(),
        }
    }

    pub fn run(&mut self, inputs: Vec<io::Result<Input>>) -> io::Result<()> {
        let requests = &mut self.requests;
        run(
            &mut self.terminal,
            &mut self.app,
            inputs.into_iter(),
            &self.opener,
            |request| requests.push(request),
        )
    }

    pub fn screen(&self) -> String {
        let buffer = self.terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|c| shown(c.symbol())).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn links(&self) -> Vec<(String, String)> {
        let buffer = self.terminal.backend().buffer();
        let width = buffer.area.width as usize;
        let mut found: Vec<(&str, String)> = Vec::new();
        for column in 0..Stage::ALL.len() {
            let span = column * width / 4..(column + 1) * width / 4;
            for row in buffer.content.chunks(width) {
                let mut open = false;
                for cell in &row[span.clone()] {
                    let Some((target, text)) = hyperlink(cell.symbol()) else {
                        open = false;
                        continue;
                    };
                    match found.last_mut().filter(|last| open && last.0 == target) {
                        Some(last) => last.1.push_str(text),
                        None => found.push((target, text.to_string())),
                    }
                    open = true;
                }
            }
        }
        found
            .into_iter()
            .map(|(target, text)| {
                let url = target.split_once(';').map_or("", |(_, url)| url);
                (text, url.to_string())
            })
            .collect()
    }

    pub fn cell(&self, x: u16, y: u16) -> &ratatui::buffer::Cell {
        &self.terminal.backend().buffer()[(x, y)]
    }
}

fn hyperlink(symbol: &str) -> Option<(&str, &str)> {
    let rest = symbol.strip_prefix("\x1b]8;")?;
    let (target, rest) = rest.split_once("\x1b\\")?;
    let text = rest.strip_suffix("\x1b]8;;\x1b\\")?;
    Some((target, text))
}

fn shown(symbol: &str) -> &str {
    hyperlink(symbol).map_or(symbol, |(_, text)| text)
}

pub fn pr(number: u64, title: &str) -> PullRequest {
    PullRequest {
        number,
        title: title.to_string(),
        url: format!("https://github.com/acme/webapp/pull/{number}"),
        repo: "acme/webapp".to_string(),
        checks: CheckState::Success,
        updated_at: Some(UNIX_EPOCH + Duration::from_secs(NOW - 2 * 3600)),
        ..PullRequest::default()
    }
}

pub fn merge_target(number: u64, title: &str) -> MergeTarget {
    MergeTarget {
        number,
        repo: "acme/webapp".to_string(),
        title: title.to_string(),
        head_oid: format!("8c1f2a7d{number}"),
    }
}

pub fn update_target(number: u64, title: &str, method: UpdateMethod) -> UpdateTarget {
    UpdateTarget {
        number,
        repo: "acme/webapp".to_string(),
        title: title.to_string(),
        head_oid: format!("8c1f2a7d{number}"),
        method,
    }
}

pub fn status_target(number: u64, title: &str, change: StatusChange) -> StatusTarget {
    StatusTarget {
        number,
        repo: "acme/webapp".to_string(),
        title: title.to_string(),
        head_oid: format!("8c1f2a7d{number}"),
        change,
    }
}

pub fn checkout_target(number: u64, dir: Option<&str>) -> CheckoutTarget {
    CheckoutTarget {
        number,
        repo: "acme/webapp".to_string(),
        dir: dir.map(PathBuf::from),
    }
}

fn pull_ref((number, author, title): (u64, &str, &str)) -> PullRef {
    PullRef {
        number,
        title: title.to_string(),
        author: author.to_string(),
        url: format!("https://github.com/acme/webapp/pull/{number}"),
    }
}
