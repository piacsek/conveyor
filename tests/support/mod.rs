#![allow(dead_code)]

use std::cell::RefCell;
use std::io;
use std::time::{Duration, UNIX_EPOCH};

use conveyor::app::{App, Input, Rows, Stage, run};
use conveyor::config::Config;
use conveyor::model::prs::{CheckState, PullRequest};
use conveyor::open::Opener;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub const NOW: u64 = 1_800_000_000;

#[derive(Default)]
pub struct FakeOpener {
    opened: RefCell<Vec<String>>,
    copied: RefCell<Vec<String>>,
}

impl FakeOpener {
    pub fn opened(&self) -> Vec<String> {
        self.opened.borrow().clone()
    }

    pub fn copied(&self) -> Vec<String> {
        self.copied.borrow().clone()
    }
}

impl Opener for FakeOpener {
    fn open(&self, url: &str) -> io::Result<()> {
        self.opened.borrow_mut().push(url.to_string());
        Ok(())
    }

    fn copy(&self, text: &str) -> io::Result<()> {
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

pub fn failed(stage: Stage, message: &str) -> io::Result<Input> {
    Ok(Input::Data(stage, Err(message.to_string())))
}

pub fn tick() -> io::Result<Input> {
    Ok(Input::Tick(UNIX_EPOCH + Duration::from_secs(NOW)))
}

pub struct Harness {
    pub terminal: Terminal<TestBackend>,
    pub app: App,
    pub opener: FakeOpener,
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
        }
    }

    pub fn run(&mut self, inputs: Vec<io::Result<Input>>) -> io::Result<()> {
        run(
            &mut self.terminal,
            &mut self.app,
            inputs.into_iter(),
            &self.opener,
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
    }
}
