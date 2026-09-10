use std::io;
use std::time::SystemTime;

use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::Config;
use crate::model::prs::PullRequest;
use crate::open::Opener;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Prs,
    Queue,
    Builds,
    Deployed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rows {
    Prs(Vec<PullRequest>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(KeyEvent),
    Data(Stage, Result<Rows, String>),
    Tick(SystemTime),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ColumnState<T> {
    #[default]
    Loading,
    Ready(Vec<T>),
}

pub struct App {
    pub prs: ColumnState<PullRequest>,
    pub config: Config,
    pub now: SystemTime,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self::at(SystemTime::now(), config)
    }

    pub fn at(now: SystemTime, config: Config) -> Self {
        Self {
            prs: ColumnState::Loading,
            config,
            now,
        }
    }

    pub fn receive(&mut self, stage: Stage, data: Result<Rows, String>) {
        if let (Stage::Prs, Ok(Rows::Prs(prs))) = (stage, data) {
            self.prs = ColumnState::Ready(prs);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            _ => Action::Continue,
        }
    }
}

pub fn run<B, O>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    inputs: impl Iterator<Item = io::Result<Input>>,
    opener: &O,
) -> io::Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
    O: Opener,
{
    let _ = opener;
    let mut inputs = inputs;
    loop {
        terminal
            .draw(|frame| ui::draw(frame, app))
            .map_err(io::Error::other)?;
        let Some(input) = inputs.next() else {
            return Ok(());
        };
        match input? {
            Input::Tick(now) => app.now = now,
            Input::Data(stage, data) => app.receive(stage, data),
            Input::Key(key) => match app.handle_key(key) {
                Action::Quit => return Ok(()),
                Action::Continue => {}
            },
        }
    }
}
