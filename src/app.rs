use std::io;
use std::time::SystemTime;

use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

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
    Open(String),
    Copy(String),
    Refresh(Stage),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Filter(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ColumnState<T> {
    #[default]
    Loading,
    Ready {
        rows: Vec<T>,
        fetched_at: SystemTime,
    },
}

impl<T> ColumnState<T> {
    pub fn into_rows(self) -> Vec<T> {
        match self {
            ColumnState::Ready { rows, .. } => rows,
            ColumnState::Loading => Vec::new(),
        }
    }
}

pub struct App {
    pub prs: ColumnState<PullRequest>,
    pub prs_error: Option<String>,
    pub list: ListState,
    pub mode: Mode,
    pub config: Config,
    pub now: SystemTime,
    pub notice: Option<String>,
    pub details: bool,
    pending_g: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self::at(SystemTime::now(), config)
    }

    pub fn at(now: SystemTime, config: Config) -> Self {
        Self {
            prs: ColumnState::Loading,
            prs_error: None,
            list: ListState::default().with_selected(Some(0)),
            mode: Mode::Normal,
            config,
            now,
            notice: None,
            details: false,
            pending_g: false,
        }
    }

    pub fn receive(&mut self, stage: Stage, data: Result<Rows, String>) {
        match (stage, data) {
            (Stage::Prs, Ok(Rows::Prs(prs))) => {
                let selected = self.selected().map(|pr| pr.number);
                let current = std::mem::take(&mut self.prs).into_rows();
                self.prs = ColumnState::Ready {
                    rows: merge_keeping_order(current, prs),
                    fetched_at: self.now,
                };
                self.prs_error = None;
                let index = selected
                    .and_then(|number| self.visible().iter().position(|pr| pr.number == number))
                    .unwrap_or(0);
                self.list.select(Some(index));
            }
            (Stage::Prs, Err(message)) => self.prs_error = Some(message),
            _ => {}
        }
    }

    pub fn filter(&self) -> Option<&str> {
        match &self.mode {
            Mode::Filter(query) => Some(query),
            Mode::Normal => None,
        }
    }

    pub fn all(&self) -> &[PullRequest] {
        match &self.prs {
            ColumnState::Ready { rows, .. } => rows,
            ColumnState::Loading => &[],
        }
    }

    pub fn fetched_at(&self) -> Option<SystemTime> {
        match &self.prs {
            ColumnState::Ready { fetched_at, .. } => Some(*fetched_at),
            ColumnState::Loading => None,
        }
    }

    pub fn visible(&self) -> Vec<&PullRequest> {
        let query = self.filter().unwrap_or("").to_lowercase();
        self.all().iter().filter(|pr| matches(pr, &query)).collect()
    }

    pub fn selected(&self) -> Option<&PullRequest> {
        self.list
            .selected()
            .and_then(|i| self.visible().get(i).copied())
    }

    fn select_next(&mut self) {
        let last = self.visible().len().saturating_sub(1);
        let next = self.list.selected().map_or(0, |i| (i + 1).min(last));
        self.list.select(Some(next));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.notice = None;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match self.mode {
            Mode::Filter(_) => self.handle_filter_key(key),
            Mode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char(c) => self.edit_filter(|q| q.push(c)),
            KeyCode::Backspace => self.edit_filter(|q| {
                q.pop();
            }),
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.list.select_first();
                Action::Continue
            }
            _ => self.handle_normal_key(key),
        }
    }

    fn edit_filter(&mut self, edit: impl FnOnce(&mut String)) -> Action {
        if let Mode::Filter(query) = &mut self.mode {
            edit(query);
        }
        self.list.select_first();
        Action::Continue
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Action {
        let pending_g = std::mem::take(&mut self.pending_g);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
            KeyCode::Char('/') => self.mode = Mode::Filter(String::new()),
            KeyCode::Enter | KeyCode::Char('o') => {
                if let Some(pr) = self.selected() {
                    return Action::Open(pr.url.clone());
                }
            }
            KeyCode::Char('y') => {
                if let Some((number, url)) = self.selected().map(|pr| (pr.number, pr.url.clone())) {
                    self.notice = Some(format!("copied #{number}"));
                    return Action::Copy(url);
                }
            }
            KeyCode::Char('r') => return Action::Refresh(Stage::Prs),
            KeyCode::Char('p') => self.details = !self.details,
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.list.select_previous(),
            KeyCode::Char('G') => self.list.select(self.visible().len().checked_sub(1)),
            KeyCode::Char('g') if pending_g => self.list.select_first(),
            KeyCode::Char('g') => self.pending_g = true,
            _ => {}
        }
        Action::Continue
    }
}

fn merge_keeping_order(current: Vec<PullRequest>, fresh: Vec<PullRequest>) -> Vec<PullRequest> {
    let mut fresh = fresh;
    let mut merged: Vec<PullRequest> = current
        .iter()
        .filter_map(|old| {
            let index = fresh.iter().position(|new| new.number == old.number)?;
            Some(fresh.remove(index))
        })
        .collect();
    merged.extend(fresh);
    merged
}

fn attempt(app: &mut App, result: io::Result<()>) {
    if let Err(err) = result {
        app.notice = Some(err.to_string());
    }
}

fn matches(pr: &PullRequest, query: &str) -> bool {
    [
        format!("#{}", pr.number),
        pr.title.to_lowercase(),
        pr.repo.to_lowercase(),
    ]
    .iter()
    .any(|text| text.contains(query))
}

pub fn run<B, O, R>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    inputs: impl Iterator<Item = io::Result<Input>>,
    opener: &O,
    mut refresh: R,
) -> io::Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
    O: Opener,
    R: FnMut(Stage),
{
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
                Action::Open(url) => attempt(app, opener.open(&url)),
                Action::Copy(text) => attempt(app, opener.copy(&text)),
                Action::Refresh(stage) => refresh(stage),
                Action::Continue => {}
            },
        }
    }
}
