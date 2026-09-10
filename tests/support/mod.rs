#![allow(dead_code)]

use std::cell::RefCell;
use std::io;
use std::time::{Duration, UNIX_EPOCH};

use conveyor::app::{App, Input, Rows, Stage, run};
use conveyor::config::Config;
use conveyor::model::builds::{Build, BuildStatus, Builds, PullRef};
use conveyor::model::deployed::{Deployed, Deployment};
use conveyor::model::prs::{CheckState, PullRequest};
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
    Ok(Input::Data(Stage::Prs, Ok(Rows::Prs(prs))))
}

pub fn queue(entries: Vec<QueueEntry>) -> io::Result<Input> {
    Ok(Input::Data(
        Stage::Queue,
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
        pull: pr.map(|(number, author, title)| PullRef {
            number,
            title: title.to_string(),
            author: author.to_string(),
            url: format!("https://github.com/acme/webapp/pull/{number}"),
        }),
    }
}

pub fn deployed(rows: Vec<Deployment>) -> io::Result<Input> {
    Ok(Input::Data(
        Stage::Deployed,
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
        pull: pr.map(|(number, author, title)| PullRef {
            number,
            title: title.to_string(),
            author: author.to_string(),
            url: format!("https://github.com/acme/webapp/pull/{number}"),
        }),
        error: None,
        fetched_at: Some(UNIX_EPOCH + Duration::from_secs(NOW - 6 * 86_400)),
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
        run_url: None,
    }
}

pub fn failed(stage: Stage, message: &str) -> io::Result<Input> {
    Ok(Input::Data(stage, Err(message.to_string())))
}

pub fn fetching(stage: Stage) -> io::Result<Input> {
    Ok(Input::Fetching(stage))
}

pub fn tick_at_ms(millis: u64) -> io::Result<Input> {
    Ok(Input::Tick(UNIX_EPOCH + Duration::from_millis(millis)))
}

pub fn tick() -> io::Result<Input> {
    tick_at(NOW)
}

pub fn tick_at(secs: u64) -> io::Result<Input> {
    Ok(Input::Tick(UNIX_EPOCH + Duration::from_secs(secs)))
}

pub struct Harness {
    pub terminal: Terminal<TestBackend>,
    pub app: App,
    pub opener: FakeOpener,
    pub refreshed: Vec<Stage>,
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
            refreshed: Vec::new(),
        }
    }

    pub fn run(&mut self, inputs: Vec<io::Result<Input>>) -> io::Result<()> {
        let refreshed = &mut self.refreshed;
        run(
            &mut self.terminal,
            &mut self.app,
            inputs.into_iter(),
            &self.opener,
            |stage| refreshed.push(stage),
        )
    }

    pub fn screen(&self) -> String {
        let buffer = self.terminal.backend().buffer();
        let width = buffer.area.width as usize;
        buffer
            .content
            .chunks(width)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn cell(&self, x: u16, y: u16) -> &ratatui::buffer::Cell {
        &self.terminal.backend().buffer()[(x, y)]
    }
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
