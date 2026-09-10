use std::io;
use std::time::{Duration, SystemTime};

pub const OPEN_DEBOUNCE: Duration = Duration::from_secs(1);

use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::config::Config;
use crate::model::builds::{Build, Builds};
use crate::model::prs::PullRequest;
use crate::model::queue::{Queue, QueueEntry};
use crate::open::Opener;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Prs,
    Queue,
    Builds,
    Deployed,
}

impl Stage {
    pub const ALL: [Stage; 4] = [Stage::Prs, Stage::Queue, Stage::Builds, Stage::Deployed];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    fn at(index: usize) -> Stage {
        Self::ALL[index.rem_euclid(Self::ALL.len())]
    }

    pub fn next(self) -> Stage {
        Self::at(self.index() + 1)
    }

    pub fn previous(self) -> Stage {
        Self::at(self.index() + Self::ALL.len() - 1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rows {
    Prs(Vec<PullRequest>),
    Queue(Queue),
    Builds(Builds),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(KeyEvent),
    Fetching(Stage),
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
    Help,
}

pub trait Row: Clone {
    fn key(&self) -> u64;
    fn url(&self) -> &str;
    fn matches(&self, query: &str) -> bool;
}

impl Row for PullRequest {
    fn key(&self) -> u64 {
        self.number
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn matches(&self, query: &str) -> bool {
        [
            format!("#{}", self.number),
            self.title.to_lowercase(),
            self.repo.to_lowercase(),
        ]
        .iter()
        .any(|text| text.contains(query))
    }
}

impl Row for QueueEntry {
    fn key(&self) -> u64 {
        self.number
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn matches(&self, query: &str) -> bool {
        [
            format!("#{}", self.number),
            self.title.to_lowercase(),
            self.author.to_lowercase(),
        ]
        .iter()
        .any(|text| text.contains(query))
    }
}

impl Row for Build {
    fn key(&self) -> u64 {
        self.id
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn matches(&self, query: &str) -> bool {
        let pull = self.pull.as_ref();
        [
            self.pr_number
                .or(pull.map(|p| p.number))
                .map(|n| format!("#{n}"))
                .unwrap_or_default(),
            self.title.to_lowercase(),
            pull.map(|p| p.author.to_lowercase()).unwrap_or_default(),
            self.sha.clone(),
        ]
        .iter()
        .any(|text| text.contains(query))
    }
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

#[derive(Debug, Clone)]
pub struct Column<T> {
    pub state: ColumnState<T>,
    pub error: Option<String>,
    pub refreshing: bool,
    pub list: ListState,
    pub filter: Option<String>,
}

impl<T> Default for Column<T> {
    fn default() -> Self {
        Self {
            state: ColumnState::Loading,
            error: None,
            refreshing: false,
            list: ListState::default().with_selected(Some(0)),
            filter: None,
        }
    }
}

impl<T: Row> Column<T> {
    pub fn all(&self) -> &[T] {
        match &self.state {
            ColumnState::Ready { rows, .. } => rows,
            ColumnState::Loading => &[],
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.state, ColumnState::Loading)
    }

    pub fn fetched_at(&self) -> Option<SystemTime> {
        match &self.state {
            ColumnState::Ready { fetched_at, .. } => Some(*fetched_at),
            ColumnState::Loading => None,
        }
    }

    pub fn visible(&self) -> Vec<&T> {
        let query = self.filter.clone().unwrap_or_default().to_lowercase();
        self.all()
            .iter()
            .filter(|row| row.matches(&query))
            .collect()
    }

    pub fn selected(&self) -> Option<&T> {
        self.list
            .selected()
            .and_then(|i| self.visible().get(i).copied())
    }

    pub fn receive(&mut self, rows: Vec<T>, now: SystemTime) {
        let selected = self.selected().map(Row::key);
        let current = std::mem::take(&mut self.state).into_rows();
        self.state = ColumnState::Ready {
            rows: merge_keeping_order(current, rows),
            fetched_at: now,
        };
        self.error = None;
        self.refreshing = false;
        let index = selected
            .and_then(|key| self.visible().iter().position(|row| row.key() == key))
            .unwrap_or(0);
        self.list.select(Some(index));
    }

    pub fn fail(&mut self, message: String) {
        self.error = Some(message);
        self.refreshing = false;
    }

    fn select_next(&mut self) {
        let last = self.visible().len().saturating_sub(1);
        let next = self.list.selected().map_or(0, |i| (i + 1).min(last));
        self.list.select(Some(next));
    }

    fn select_last(&mut self) {
        self.list.select(self.visible().len().checked_sub(1));
    }

    fn select_index(&mut self, index: usize) {
        if index < self.visible().len() {
            self.list.select(Some(index));
        }
    }
}

fn merge_keeping_order<T: Row>(current: Vec<T>, fresh: Vec<T>) -> Vec<T> {
    let mut fresh = fresh;
    let mut merged: Vec<T> = current
        .iter()
        .filter_map(|old| {
            let index = fresh.iter().position(|new| new.key() == old.key())?;
            Some(fresh.remove(index))
        })
        .collect();
    merged.extend(fresh);
    merged
}

pub struct App {
    pub prs: Column<PullRequest>,
    pub queue: Column<QueueEntry>,
    pub queue_repo: Option<String>,
    pub builds: Column<Build>,
    pub builds_repo: Option<String>,
    pub focus: Stage,
    pub mode: Mode,
    pub config: Config,
    pub now: SystemTime,
    pub utc_offset_secs: i32,
    pub notice: Option<String>,
    pub details: bool,
    pending_g: bool,
    last_open: Option<(String, SystemTime)>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self::at(SystemTime::now(), config)
    }

    pub fn at(now: SystemTime, config: Config) -> Self {
        Self {
            prs: Column::default(),
            queue: Column::default(),
            queue_repo: None,
            builds: Column::default(),
            builds_repo: None,
            focus: Stage::Prs,
            mode: Mode::Normal,
            config,
            now,
            utc_offset_secs: 0,
            notice: None,
            details: false,
            pending_g: false,
            last_open: None,
        }
    }

    pub fn receive(&mut self, stage: Stage, data: Result<Rows, String>) {
        match (stage, data) {
            (Stage::Prs, Ok(Rows::Prs(prs))) => self.prs.receive(prs, self.now),
            (Stage::Queue, Ok(Rows::Queue(queue))) => {
                self.queue_repo = Some(queue.repo);
                self.queue.receive(queue.entries, self.now);
            }
            (Stage::Builds, Ok(Rows::Builds(builds))) => {
                self.builds_repo = Some(builds.repo);
                self.builds.receive(builds.builds, self.now);
            }
            (Stage::Prs, Err(message)) => self.prs.fail(message),
            (Stage::Queue, Err(message)) => self.queue.fail(message),
            (Stage::Builds, Err(message)) => self.builds.fail(message),
            _ => {}
        }
    }

    pub fn fetching(&mut self, stage: Stage) {
        match stage {
            Stage::Prs => self.prs.refreshing = true,
            Stage::Queue => self.queue.refreshing = true,
            Stage::Builds => self.builds.refreshing = true,
            Stage::Deployed => {}
        }
    }

    pub fn spinner(&self) -> char {
        const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let millis = self
            .now
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        FRAMES[(millis / 100 % FRAMES.len() as u128) as usize]
    }

    pub fn filter(&self) -> Option<&str> {
        match &self.mode {
            Mode::Filter(query) => Some(query),
            Mode::Normal | Mode::Help => None,
        }
    }

    pub fn focused_error(&self) -> Option<&str> {
        match self.focus {
            Stage::Prs => self.prs.error.as_deref(),
            Stage::Queue => self.queue.error.as_deref(),
            Stage::Builds => self.builds.error.as_deref(),
            Stage::Deployed => None,
        }
    }

    pub fn focused_fetched_at(&self) -> Option<SystemTime> {
        match self.focus {
            Stage::Prs => self.prs.fetched_at(),
            Stage::Queue => self.queue.fetched_at(),
            Stage::Builds => self.builds.fetched_at(),
            Stage::Deployed => None,
        }
    }

    pub fn focused_counts(&self) -> (usize, usize) {
        match self.focus {
            Stage::Prs => (self.prs.visible().len(), self.prs.all().len()),
            Stage::Queue => (self.queue.visible().len(), self.queue.all().len()),
            Stage::Builds => (self.builds.visible().len(), self.builds.all().len()),
            Stage::Deployed => (0, 0),
        }
    }

    fn selected_target(&self) -> Option<(u64, String)> {
        match self.focus {
            Stage::Prs => self.prs.selected().map(|pr| (pr.key(), pr.url.clone())),
            Stage::Queue => self
                .queue
                .selected()
                .map(|entry| (entry.key(), entry.url.clone())),
            Stage::Builds => self.builds.selected().map(|build| {
                (
                    build
                        .pr_number
                        .or(build.pull.as_ref().map(|p| p.number))
                        .unwrap_or(build.run_number),
                    build.url.clone(),
                )
            }),
            Stage::Deployed => None,
        }
    }

    fn opened_recently(&self, url: &str) -> bool {
        self.last_open.as_ref().is_some_and(|(last, at)| {
            last == url
                && self
                    .now
                    .duration_since(*at)
                    .is_ok_and(|elapsed| elapsed < OPEN_DEBOUNCE)
        })
    }

    fn with_focused(&mut self, act: impl Fn(&mut dyn Navigable)) {
        match self.focus {
            Stage::Prs => act(&mut self.prs),
            Stage::Queue => act(&mut self.queue),
            Stage::Builds => act(&mut self.builds),
            Stage::Deployed => {}
        }
    }

    fn sync_filter(&mut self) {
        let filter = self.filter().map(str::to_string);
        self.prs.filter = filter.clone();
        self.queue.filter = filter.clone();
        self.builds.filter = filter;
        self.with_focused(|column| column.select_index_clamped(0));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.notice = None;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match self.mode {
            Mode::Help => {
                self.mode = Mode::Normal;
                Action::Continue
            }
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
                self.sync_filter();
                Action::Continue
            }
            _ => self.handle_normal_key(key),
        }
    }

    fn edit_filter(&mut self, edit: impl FnOnce(&mut String)) -> Action {
        if let Mode::Filter(query) = &mut self.mode {
            edit(query);
        }
        self.sync_filter();
        Action::Continue
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Action {
        let pending_g = std::mem::take(&mut self.pending_g);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
            KeyCode::Char('/') => {
                self.mode = Mode::Filter(String::new());
                self.sync_filter();
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Enter | KeyCode::Char('o') => {
                if let Some((_, url)) = self.selected_target()
                    && !self.opened_recently(&url)
                {
                    self.last_open = Some((url.clone(), self.now));
                    return Action::Open(url);
                }
            }
            KeyCode::Char('y') => {
                if let Some((number, url)) = self.selected_target() {
                    self.notice = Some(format!("copied #{number}"));
                    return Action::Copy(url);
                }
            }
            KeyCode::Char('r') => return Action::Refresh(self.focus),
            KeyCode::Char('p') => self.details = !self.details,
            KeyCode::Char('l') if self.focus != Stage::Deployed => self.focus = self.focus.next(),
            KeyCode::Char('h') if self.focus != Stage::Prs => self.focus = self.focus.previous(),
            KeyCode::Tab => self.focus = self.focus.next(),
            KeyCode::BackTab => self.focus = self.focus.previous(),
            KeyCode::Char(digit @ '1'..='9') => {
                let index = digit.to_digit(10).unwrap_or(1) as usize - 1;
                self.with_focused(|column| column.select_index_clamped(index));
            }
            KeyCode::Char('j') | KeyCode::Down => self.with_focused(|c| c.next()),
            KeyCode::Char('k') | KeyCode::Up => self.with_focused(|c| c.previous()),
            KeyCode::Char('G') => self.with_focused(|c| c.last()),
            KeyCode::Char('g') if pending_g => self.with_focused(|c| c.first()),
            KeyCode::Char('g') => self.pending_g = true,
            _ => {}
        }
        Action::Continue
    }
}

pub trait Navigable {
    fn next(&mut self);
    fn previous(&mut self);
    fn first(&mut self);
    fn last(&mut self);
    fn select_index_clamped(&mut self, index: usize);
}

impl<T: Row> Navigable for Column<T> {
    fn next(&mut self) {
        self.select_next();
    }

    fn previous(&mut self) {
        self.list.select_previous();
    }

    fn first(&mut self) {
        self.list.select_first();
    }

    fn last(&mut self) {
        self.select_last();
    }

    fn select_index_clamped(&mut self, index: usize) {
        if index == 0 {
            self.list.select_first();
        } else {
            self.select_index(index);
        }
    }
}

fn attempt(app: &mut App, result: io::Result<()>) {
    if let Err(err) = result {
        app.notice = Some(err.to_string());
    }
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
            Input::Fetching(stage) => app.fetching(stage),
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
