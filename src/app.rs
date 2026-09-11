use std::collections::HashMap;
use std::io;
use std::time::{Duration, SystemTime};

pub const OPEN_DEBOUNCE: Duration = Duration::from_secs(1);
const EAGER_JOBS: usize = 5;

use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::config::Config;
use crate::model::builds::{Build, BuildStatus, Builds};
use crate::model::deployed::{Deployed, Deployment};
use crate::model::jobs::Job;
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
    Deployed(Deployed),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(KeyEvent),
    Fetching(Stage),
    Data(Stage, Result<Rows, String>),
    Jobs(u64, Result<Vec<Job>, String>),
    Tick(SystemTime),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobsState {
    Loading,
    Ready(Vec<Job>),
    Failed(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Quit,
    Open(String),
    Copy(String),
    Refresh(Stage),
    RefreshAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Refresh(Stage),
    Jobs { run_id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Filter(String),
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    AsFetched,
    ByRankAscending,
    ByRankDescending,
}

pub trait Row: Clone {
    const ORDER: Order = Order::AsFetched;

    fn key(&self) -> u64;

    fn rank(&self) -> u64 {
        0
    }

    fn matches(&self, query: &str) -> bool;
}

impl Row for PullRequest {
    fn key(&self) -> u64 {
        self.number
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
    const ORDER: Order = Order::ByRankAscending;

    fn key(&self) -> u64 {
        self.number
    }

    fn rank(&self) -> u64 {
        self.position
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
    const ORDER: Order = Order::ByRankDescending;

    fn key(&self) -> u64 {
        self.id
    }

    fn rank(&self) -> u64 {
        self.run_number
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

impl Row for Deployment {
    fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        self.env.hash(&mut hasher);
        hasher.finish()
    }

    fn matches(&self, query: &str) -> bool {
        let pull = self.pull.as_ref();
        [
            self.env.to_lowercase(),
            pull.map(|p| format!("#{}", p.number)).unwrap_or_default(),
            pull.map(|p| p.title.to_lowercase()).unwrap_or_default(),
            pull.map(|p| p.author.to_lowercase()).unwrap_or_default(),
            self.sha.clone().unwrap_or_default(),
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
        let mut rows = merge_keeping_order(current, rows);
        match T::ORDER {
            Order::AsFetched => {}
            Order::ByRankAscending => rows.sort_by_key(Row::rank),
            Order::ByRankDescending => rows.sort_by_key(|row| std::cmp::Reverse(row.rank())),
        }
        self.state = ColumnState::Ready {
            rows,
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
    pub deployed: Column<Deployment>,
    pub deployed_system: Option<String>,
    pub focus: Stage,
    pub mode: Mode,
    pub config: Config,
    pub now: SystemTime,
    pub utc_offset_secs: i32,
    pub notice: Option<String>,
    pub details: bool,
    pub details_scroll: u16,
    pub details_page: u16,
    pub zoom: bool,
    pub jobs: HashMap<u64, JobsState>,
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
            deployed: Column::default(),
            deployed_system: None,
            focus: Stage::Prs,
            mode: Mode::Normal,
            config,
            now,
            utc_offset_secs: 0,
            notice: None,
            details: false,
            details_scroll: 0,
            details_page: 0,
            zoom: false,
            jobs: HashMap::new(),
            pending_g: false,
            last_open: None,
        }
    }

    pub fn receive(&mut self, stage: Stage, data: Result<Rows, String>) {
        let selected = self.selected_key();
        self.receive_rows(stage, data);
        if self.selected_key() != selected {
            self.details_scroll = 0;
        }
    }

    fn selected_key(&self) -> Option<u64> {
        match self.focus {
            Stage::Prs => self.prs.selected().map(Row::key),
            Stage::Queue => self.queue.selected().map(Row::key),
            Stage::Builds => self.builds.selected().map(Row::key),
            Stage::Deployed => self.deployed.selected().map(Row::key),
        }
    }

    fn receive_rows(&mut self, stage: Stage, data: Result<Rows, String>) {
        match (stage, data) {
            (Stage::Prs, Ok(Rows::Prs(prs))) => self.prs.receive(prs, self.now),
            (Stage::Queue, Ok(Rows::Queue(queue))) => {
                self.queue_repo = Some(queue.repo);
                self.queue.receive(queue.entries, self.now);
                self.forget_unsettled_jobs();
            }
            (Stage::Builds, Ok(Rows::Builds(builds))) => {
                self.builds_repo = Some(builds.repo);
                self.builds.receive(builds.builds, self.now);
                self.forget_unsettled_jobs();
            }
            (Stage::Deployed, Ok(Rows::Deployed(deployed))) => {
                self.deployed_system = Some(deployed.system);
                let rows = keep_last_known(self.deployed.all(), deployed.rows);
                self.deployed.receive(rows, self.now);
            }
            (Stage::Prs, Err(message)) => self.prs.fail(message),
            (Stage::Queue, Err(message)) => self.queue.fail(message),
            (Stage::Builds, Err(message)) => self.builds.fail(message),
            (Stage::Deployed, Err(message)) => self.deployed.fail(message),
            _ => {}
        }
    }

    fn forget_unsettled_jobs(&mut self) {
        let queued = self
            .queue
            .all()
            .iter()
            .filter_map(|entry| entry.run.as_ref());
        let unsettled: Vec<u64> = self
            .builds
            .all()
            .iter()
            .chain(queued)
            .filter(|build| !build.is_settled())
            .map(|build| build.id)
            .collect();
        for run_id in unsettled {
            self.jobs.remove(&run_id);
        }
    }

    pub fn selected_run(&self) -> Option<&Build> {
        match self.focus {
            Stage::Builds => self.builds.selected(),
            Stage::Queue => self.queue.selected()?.run.as_ref(),
            _ => None,
        }
    }

    pub fn jobs_needed(&mut self) -> Option<u64> {
        let selected = self.selected_run().map(|run| run.id);
        let run_id = selected
            .filter(|run_id| !self.jobs.contains_key(run_id))
            .or_else(|| self.failing_run_without_jobs())?;
        self.jobs.insert(run_id, JobsState::Loading);
        Some(run_id)
    }

    fn failing_run_without_jobs(&self) -> Option<u64> {
        self.builds
            .all()
            .iter()
            .filter(|build| build.status == BuildStatus::Failure)
            .take(EAGER_JOBS)
            .find(|build| !self.jobs.contains_key(&build.id))
            .map(|build| build.id)
    }

    pub fn selected_jobs(&self) -> Option<&JobsState> {
        self.jobs.get(&self.selected_run()?.id)
    }

    pub fn receive_jobs(&mut self, run_id: u64, result: Result<Vec<Job>, String>) {
        let state = match result {
            Ok(jobs) => JobsState::Ready(jobs),
            Err(message) => JobsState::Failed(message),
        };
        self.jobs.insert(run_id, state);
    }

    pub fn fetching(&mut self, stage: Stage) {
        match stage {
            Stage::Prs => self.prs.refreshing = true,
            Stage::Queue => self.queue.refreshing = true,
            Stage::Builds => self.builds.refreshing = true,
            Stage::Deployed => self.deployed.refreshing = true,
        }
    }

    pub fn any_refreshing(&self) -> bool {
        self.prs.refreshing
            || self.queue.refreshing
            || self.builds.refreshing
            || self.deployed.refreshing
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
            Stage::Deployed => self.deployed.error.as_deref().or_else(|| {
                self.deployed
                    .selected()
                    .and_then(|row| row.error.as_deref())
            }),
        }
    }

    pub fn focused_fetched_at(&self) -> Option<SystemTime> {
        match self.focus {
            Stage::Prs => self.prs.fetched_at(),
            Stage::Queue => self.queue.fetched_at(),
            Stage::Builds => self.builds.fetched_at(),
            Stage::Deployed => self.deployed.fetched_at(),
        }
    }

    pub fn focused_counts(&self) -> (usize, usize) {
        match self.focus {
            Stage::Prs => (self.prs.visible().len(), self.prs.all().len()),
            Stage::Queue => (self.queue.visible().len(), self.queue.all().len()),
            Stage::Builds => (self.builds.visible().len(), self.builds.all().len()),
            Stage::Deployed => (self.deployed.visible().len(), self.deployed.all().len()),
        }
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        let nothing = || "nothing selected".to_string();
        match self.focus {
            Stage::Prs => self
                .prs
                .selected()
                .map(|pr| (pr.key(), pr.url.clone()))
                .ok_or_else(nothing),
            Stage::Queue => self
                .queue
                .selected()
                .map(|entry| (entry.key(), entry.url.clone()))
                .ok_or_else(nothing),
            Stage::Builds => {
                let build = self.builds.selected().ok_or_else(nothing)?;
                match (&build.pull, build.pr_number, &self.builds_repo) {
                    (Some(pull), _, _) => Ok((pull.number, pull.url.clone())),
                    (None, Some(number), Some(repo)) => {
                        Ok((number, format!("https://github.com/{repo}/pull/{number}")))
                    }
                    _ => Err("no pull request for this run".to_string()),
                }
            }
            Stage::Deployed => self
                .deployed
                .selected()
                .ok_or_else(nothing)?
                .pull
                .as_ref()
                .map(|pull| (pull.number, pull.url.clone()))
                .ok_or_else(|| "no pull request for this commit".to_string()),
        }
    }

    fn build_url(&self) -> Result<String, String> {
        match self.focus {
            Stage::Prs => self
                .prs
                .selected()
                .and_then(|pr| {
                    pr.checks_failures_first()
                        .into_iter()
                        .find(|check| !check.url.is_empty())
                        .map(|check| check.url.clone())
                })
                .ok_or_else(|| "no checks yet".to_string()),
            Stage::Queue => self
                .queue
                .selected()
                .and_then(|entry| entry.run.as_ref())
                .map(|run| run.url.clone())
                .ok_or_else(|| "no merge-group run yet".to_string()),
            Stage::Builds => self
                .builds
                .selected()
                .map(|build| build.url.clone())
                .ok_or_else(|| "nothing selected".to_string()),
            Stage::Deployed => {
                let sha = self
                    .deployed
                    .selected()
                    .and_then(|row| row.sha.clone())
                    .ok_or_else(|| "no deployed sha to look up".to_string())?;
                self.builds
                    .all()
                    .iter()
                    .find(|build| build.sha == sha)
                    .map(|build| build.url.clone())
                    .ok_or_else(|| {
                        format!(
                            "no main build found for {}",
                            sha.chars().take(8).collect::<String>()
                        )
                    })
            }
        }
    }

    pub fn behind_main(&self, row: &Deployment) -> Option<usize> {
        let sha = row.sha.as_ref()?;
        self.builds.all().iter().position(|build| &build.sha == sha)
    }

    pub fn deploy_configured(&self) -> bool {
        self.config.repo.iter().any(|repo| !repo.deploy.is_empty())
    }

    fn open(&mut self, url: String) -> Action {
        if self.opened_recently(&url) {
            return Action::Continue;
        }
        self.last_open = Some((url.clone(), self.now));
        Action::Open(url)
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

    fn refocus(&mut self, step: impl Fn(Stage) -> Stage) {
        self.focus = step(self.focus);
        self.details_scroll = 0;
    }

    fn scroll_details(&mut self, half_pages: i32) {
        let step = (self.details_page / 2).max(1) as i32;
        let scroll = i32::from(self.details_scroll) + half_pages * step;
        self.details_scroll = scroll.clamp(0, i32::from(u16::MAX)) as u16;
    }

    fn with_focused(&mut self, act: impl Fn(&mut dyn Navigable)) {
        self.details_scroll = 0;
        match self.focus {
            Stage::Prs => act(&mut self.prs),
            Stage::Queue => act(&mut self.queue),
            Stage::Builds => act(&mut self.builds),
            Stage::Deployed => act(&mut self.deployed),
        }
    }

    fn sync_filter(&mut self) {
        let filter = self.filter().map(str::to_string);
        self.prs.filter = filter.clone();
        self.queue.filter = filter.clone();
        self.builds.filter = filter.clone();
        self.deployed.filter = filter;
        self.with_focused(|column| column.select_index_clamped(0));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.notice = None;
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if control && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match self.mode {
            Mode::Help => {
                self.mode = Mode::Normal;
                Action::Continue
            }
            _ if control && key.code == KeyCode::Char('d') => {
                self.scroll_details(1);
                Action::Continue
            }
            _ if control && key.code == KeyCode::Char('u') => {
                self.scroll_details(-1);
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
            KeyCode::Char('q') => return Action::Quit,
            KeyCode::Esc => self.zoom = false,
            KeyCode::Char('/') => {
                self.mode = Mode::Filter(String::new());
                self.sync_filter();
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Char('p') => match self.pull_target() {
                Ok((_, url)) => return self.open(url),
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('b') => match self.build_url() {
                Ok(url) => return self.open(url),
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('y') => match self.pull_target() {
                Ok((number, url)) => {
                    self.notice = Some(format!("copied #{number}"));
                    return Action::Copy(url);
                }
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('Y') => match self.build_url() {
                Ok(url) => {
                    self.notice = Some("copied the build URL".to_string());
                    return Action::Copy(url);
                }
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('r') => return Action::Refresh(self.focus),
            KeyCode::Char('R') => return Action::RefreshAll,
            KeyCode::Char('d') => {
                self.details = !self.details;
                self.details_scroll = 0;
            }
            KeyCode::Char('z') => self.zoom = !self.zoom,
            KeyCode::Char('l') if self.focus != Stage::Deployed => self.refocus(Stage::next),
            KeyCode::Char('h') if self.focus != Stage::Prs => self.refocus(Stage::previous),
            KeyCode::Tab => self.refocus(Stage::next),
            KeyCode::BackTab => self.refocus(Stage::previous),
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

fn keep_last_known(current: &[Deployment], fresh: Vec<Deployment>) -> Vec<Deployment> {
    fresh
        .into_iter()
        .map(|mut row| {
            if row.sha.is_none()
                && let Some(known) = current.iter().find(|old| old.env == row.env)
            {
                row.sha = known.sha.clone();
                row.image = known.image.clone();
                row.pull = known.pull.clone();
                row.fetched_at = known.fetched_at;
            }
            row
        })
        .collect()
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
    mut request: R,
) -> io::Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
    O: Opener,
    R: FnMut(Request),
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
            Input::Jobs(run_id, result) => app.receive_jobs(run_id, result),
            Input::Key(key) => match app.handle_key(key) {
                Action::Quit => return Ok(()),
                Action::Open(url) => attempt(app, opener.open(&url)),
                Action::Copy(text) => attempt(app, opener.copy(&text)),
                Action::Refresh(stage) => request(Request::Refresh(stage)),
                Action::RefreshAll => {
                    for stage in Stage::ALL {
                        request(Request::Refresh(stage));
                    }
                }
                Action::Continue => {}
            },
        }
        if let Some(run_id) = app.jobs_needed() {
            request(Request::Jobs { run_id });
        }
    }
}
