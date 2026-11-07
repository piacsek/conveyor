use std::collections::HashMap;
use std::io;
use std::time::{Duration, SystemTime};

pub const OPEN_DEBOUNCE: Duration = Duration::from_secs(1);
const EAGER_JOBS: usize = 5;
pub const FOLLOW_UP: Duration = Duration::from_secs(10);

use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::config::{Config, UpdateMethod};
use crate::model::builds::{Build, BuildStatus, Builds};
use crate::model::deployed::{Deployed, Deployment};
use crate::model::jobs::Job;
use crate::model::prs::{BranchUpdate, MergeState, PullRequest, StatusChange};
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
    /// The tail of one job's failed-step log, keyed by job id.
    Log(u64, Result<Vec<String>, String>),
    Merged(u64, Result<(), String>),
    Rerun(Vec<(u64, Result<(), String>)>),
    BranchUpdated(u64, Result<BranchUpdate, String>),
    StatusChanged(u64, StatusChange, Result<(), String>),
    Tick(SystemTime),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobsState {
    Loading,
    Ready(Vec<Job>),
    Stale(Vec<Job>),
    Refreshing(Vec<Job>),
    Failed(String),
}

impl JobsState {
    pub fn known(&self) -> Option<&[Job]> {
        match self {
            JobsState::Ready(jobs) | JobsState::Stale(jobs) | JobsState::Refreshing(jobs) => {
                Some(jobs)
            }
            JobsState::Loading | JobsState::Failed(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogState {
    Loading,
    Ready(Vec<String>),
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
    Log { run_id: u64, job_id: u64 },
    Rerun { repo: String, run_ids: Vec<u64> },
    Merge(MergeTarget),
    UpdateBranch(UpdateTarget),
    ChangeStatus(StatusTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub change: StatusChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Merge(MergeTarget),
    Update(UpdateTarget),
    Rerun(Rerun),
    Status(StatusTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub key: char,
    pub label: String,
    pub choice: Result<Choice, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub title: String,
    pub offers: Vec<Offer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub method: UpdateMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Refresh(Stage),
    Jobs { run_id: u64 },
    Log { run_id: u64, job_id: u64 },
    Rerun { repo: String, run_ids: Vec<u64> },
    Merge(MergeTarget),
    UpdateBranch(UpdateTarget),
    ChangeStatus(StatusTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rerun {
    pub repo: String,
    pub run_ids: Vec<u64>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    Merge(MergeTarget),
    UpdateBranch(UpdateTarget),
    Status(StatusTarget),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Filter(String),
    Help,
    Changelog,
    Confirm(Pending),
    Manage(Menu),
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
    const ORDER: Order = Order::ByRankAscending;

    fn key(&self) -> u64 {
        self.number
    }

    fn rank(&self) -> u64 {
        match (self.queue_position, self.is_draft) {
            (Some(position), _) => position,
            (None, false) => u64::MAX - 1,
            (None, true) => u64::MAX,
        }
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
    pub merging: Option<u64>,
    pub updating: Option<u64>,
    pub follow_up: Option<SystemTime>,
    pub merge_landing: Option<(u64, bool)>,
    pub details: bool,
    pub details_scroll: u16,
    pub details_page: u16,
    pub changelog: &'static str,
    pub changelog_scroll: u16,
    pub changelog_page: u16,
    pub zoom: bool,
    pub jobs: HashMap<u64, JobsState>,
    /// Failed-step log tails by job id, and whether the pane shows one for the selected run.
    pub logs: HashMap<u64, LogState>,
    pub log_open: bool,
    /// The log pane's scroll; `u16::MAX` means "the end", clamped at draw like the details.
    pub log_scroll: u16,
    pub log_page: u16,
    /// The live filter applied to every column; `Mode::Filter` means it is being edited.
    pub query: Option<String>,
    rerunning: Option<(Stage, String)>,
    changing: Option<(u64, StatusChange)>,
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
            merging: None,
            updating: None,
            follow_up: None,
            merge_landing: None,
            details: false,
            details_scroll: 0,
            details_page: 0,
            changelog: crate::changelog::CHANGELOG,
            changelog_scroll: 0,
            changelog_page: 0,
            zoom: false,
            jobs: HashMap::new(),
            logs: HashMap::new(),
            log_open: false,
            log_scroll: u16::MAX,
            log_page: 0,
            query: None,
            rerunning: None,
            changing: None,
            pending_g: false,
            last_open: None,
        }
    }

    pub fn receive(&mut self, stage: Stage, data: Result<Rows, String>) {
        if stage == Stage::Prs && matches!(self.merge_landing, Some((_, true))) {
            self.merge_landing = None;
        }
        let selected = self.selected_key();
        self.receive_rows(stage, data);
        if self.selected_key() != selected {
            self.reset_scroll();
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
                self.age_jobs(Stage::Queue);
            }
            (Stage::Builds, Ok(Rows::Builds(builds))) => {
                self.builds_repo = Some(builds.repo);
                self.builds.receive(builds.builds, self.now);
                self.age_jobs(Stage::Builds);
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

    fn age_jobs(&mut self, stage: Stage) {
        let runs: Vec<(u64, bool, u64)> = match stage {
            Stage::Queue => self
                .queue
                .all()
                .iter()
                .filter_map(|entry| entry.run.as_ref())
                .map(|run| (run.id, run.is_settled(), run.attempt))
                .collect(),
            Stage::Builds => self
                .builds
                .all()
                .iter()
                .map(|build| (build.id, build.is_settled(), build.attempt))
                .collect(),
            Stage::Prs | Stage::Deployed => Vec::new(),
        };
        for (run_id, settled, attempt) in runs {
            let aged = match (self.jobs.remove(&run_id), settled) {
                (Some(JobsState::Ready(jobs)), false) => Some(JobsState::Stale(jobs)),
                (Some(JobsState::Failed(_)), false) => None,
                (Some(JobsState::Ready(jobs)), true)
                    if jobs.iter().any(|job| !job.status.is_settled())
                        || jobs
                            .iter()
                            .map(|job| job.attempt)
                            .max()
                            .is_some_and(|newest| newest < attempt) =>
                {
                    Some(JobsState::Stale(jobs))
                }
                (kept, _) => kept,
            };
            if let Some(state) = aged {
                self.jobs.insert(run_id, state);
            }
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
            .filter(|run_id| matches!(self.jobs.get(run_id), None | Some(JobsState::Stale(_))))
            .or_else(|| self.stale_run())
            .or_else(|| self.failing_run_without_jobs())?;
        let state = match self.jobs.remove(&run_id) {
            Some(JobsState::Stale(jobs)) => JobsState::Refreshing(jobs),
            _ => JobsState::Loading,
        };
        self.jobs.insert(run_id, state);
        Some(run_id)
    }

    fn run_is_settled(&self, run_id: u64) -> bool {
        self.builds
            .all()
            .iter()
            .chain(
                self.queue
                    .all()
                    .iter()
                    .filter_map(|entry| entry.run.as_ref()),
            )
            .any(|run| run.id == run_id && run.is_settled())
    }

    fn stale_run(&self) -> Option<u64> {
        self.jobs
            .iter()
            .filter(|(_, state)| matches!(state, JobsState::Stale(_)))
            .map(|(run_id, _)| *run_id)
            .min()
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

    /// The first failed job of the selected run, once its jobs are known.
    pub fn selected_failed_job(&self) -> Option<&Job> {
        self.selected_jobs()?
            .known()?
            .iter()
            .find(|job| job.status == BuildStatus::Failure)
    }

    /// The log the pane shows for the selected run: only while `L` has it open.
    pub fn selected_log(&self) -> Option<(&Job, &LogState)> {
        if !self.log_open {
            return None;
        }
        let job = self.selected_failed_job()?;
        Some((job, self.logs.get(&job.id)?))
    }

    /// `L` hides the log when one is on screen; otherwise it opens the pane and shows (fetching
    /// if needed) the selected run's failed-step log.
    fn toggle_log(&mut self) -> Action {
        if self.log_open && self.details && self.selected_log().is_some() {
            self.log_open = false;
            return Action::Continue;
        }
        let Some(run_id) = self.selected_run().map(|run| run.id) else {
            self.notice = Some("nothing selected".to_string());
            return Action::Continue;
        };
        let Some(job_id) = self.selected_failed_job().map(|job| job.id) else {
            self.notice = Some("no failed job to show".to_string());
            return Action::Continue;
        };
        self.log_open = true;
        self.details = true;
        self.log_scroll = u16::MAX;
        // A failed fetch is not cached: the next `L` asks again.
        if matches!(
            self.logs.get(&job_id),
            Some(LogState::Loading | LogState::Ready(_))
        ) {
            return Action::Continue;
        }
        self.logs.insert(job_id, LogState::Loading);
        Action::Log { run_id, job_id }
    }

    pub fn receive_log(&mut self, job_id: u64, result: Result<Vec<String>, String>) {
        let state = match result {
            Ok(lines) => LogState::Ready(lines),
            Err(message) => LogState::Failed(message),
        };
        self.logs.insert(job_id, state);
        // Only the log on screen jumps to its end; a late one for another run stays quiet.
        if self.selected_failed_job().map(|job| job.id) == Some(job_id) {
            self.log_scroll = u16::MAX;
        }
    }

    pub fn receive_jobs(&mut self, run_id: u64, result: Result<Vec<Job>, String>) {
        let settled = self.run_is_settled(run_id);
        let state = match (result, self.jobs.remove(&run_id)) {
            (Ok(jobs), _) => JobsState::Ready(jobs),
            (Err(_), Some(JobsState::Refreshing(jobs))) if !settled => JobsState::Ready(jobs),
            (Err(message), _) => JobsState::Failed(message),
        };
        self.jobs.insert(run_id, state);
    }

    pub fn fetching(&mut self, stage: Stage) {
        if let (Stage::Prs, Some((_, started))) = (stage, self.merge_landing.as_mut()) {
            *started = true;
        }
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
        self.query.as_deref()
    }

    pub fn editing_filter(&self) -> bool {
        matches!(self.mode, Mode::Filter(_))
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
                match self.builds.all().iter().find(|build| build.sha == sha) {
                    Some(build) => Ok(build.url.clone()),
                    // Older than the fetched main builds: the commit page still shows its
                    // checks and the run behind them.
                    None => self
                        .deploy_repo()
                        .map(|repo| format!("https://github.com/{repo}/commit/{sha}"))
                        .ok_or_else(|| {
                            format!(
                                "no main build found for {}",
                                sha.chars().take(8).collect::<String>()
                            )
                        }),
                }
            }
        }
    }

    /// The repository the Deployed column watches: the configured one with a `[[repo.deploy]]`,
    /// else the one the main builds came from.
    fn deploy_repo(&self) -> Option<String> {
        self.config
            .repo
            .iter()
            .find(|repo| !repo.deploy.is_empty())
            .map(|repo| repo.name.clone())
            .or_else(|| self.builds_repo.clone())
    }

    pub fn behind_main(&self, row: &Deployment) -> Option<usize> {
        let sha = row.sha.as_ref()?;
        self.builds.all().iter().position(|build| &build.sha == sha)
    }

    pub fn deploy_configured(&self) -> bool {
        self.config.repo.iter().any(|repo| !repo.deploy.is_empty())
    }

    fn open(&mut self, url: String) -> Action {
        if let Err(err) = crate::open::web_url(&url) {
            self.notice = Some(err.to_string());
            return Action::Continue;
        }
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
        self.reset_scroll();
    }

    /// Both panes start at their resting place: the details at the top, the log at its end.
    fn reset_scroll(&mut self) {
        self.details_scroll = 0;
        self.log_scroll = u16::MAX;
    }

    /// `Ctrl-d`/`Ctrl-u` move the log pane while it is open, else the details pane.
    fn scroll_details(&mut self, half_pages: i32) {
        let (scroll, page) = if self.details && self.selected_log().is_some() {
            (&mut self.log_scroll, self.log_page)
        } else {
            (&mut self.details_scroll, self.details_page)
        };
        let step = (page / 2).max(1) as i32;
        let next = i32::from(*scroll) + half_pages * step;
        *scroll = next.clamp(0, i32::from(u16::MAX)) as u16;
    }

    fn half_changelog(&self) -> i32 {
        (self.changelog_page / 2).max(1) as i32
    }

    fn scroll_changelog(&mut self, lines: i32) {
        let next = i32::from(self.changelog_scroll) + lines;
        self.changelog_scroll = next.clamp(0, i32::from(u16::MAX)) as u16;
    }

    fn with_focused(&mut self, act: impl Fn(&mut dyn Navigable)) {
        self.reset_scroll();
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
            Mode::Changelog => {
                match key.code {
                    KeyCode::Char('d') if control => self.scroll_changelog(self.half_changelog()),
                    KeyCode::Char('u') if control => self.scroll_changelog(-self.half_changelog()),
                    KeyCode::Char('j') | KeyCode::Down => self.scroll_changelog(1),
                    KeyCode::Char('k') | KeyCode::Up => self.scroll_changelog(-1),
                    _ => self.mode = Mode::Normal,
                }
                Action::Continue
            }
            Mode::Confirm(_) => self.confirm(key),
            Mode::Manage(_) => self.manage(key),
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

    fn confirm(&mut self, key: KeyEvent) -> Action {
        let Mode::Confirm(pending) = std::mem::take(&mut self.mode) else {
            return Action::Continue;
        };
        match pending {
            Pending::Merge(target) => self.confirm_merge(key, target),
            Pending::UpdateBranch(target) => self.confirm_update(key, target),
            Pending::Status(target) => self.confirm_status(key, target),
        }
    }

    fn ask_to_manage(&mut self) {
        if let Some(busy) = self.write_in_flight() {
            self.notice = Some(busy);
            return;
        }
        match self.menu() {
            Ok(menu) => self.mode = Mode::Manage(menu),
            Err(message) => self.notice = Some(message),
        }
    }

    fn write_in_flight(&self) -> Option<String> {
        if let Some(number) = self.merging {
            return Some(format!("merging #{number}…"));
        }
        if let Some(number) = self.updating {
            return Some(format!("updating #{number}…"));
        }
        if let Some((number, change)) = &self.changing {
            return Some(progress(*number, change));
        }
        self.rerunning
            .as_ref()
            .map(|(_, label)| format!("rerunning {label}…"))
    }

    fn menu(&self) -> Result<Menu, String> {
        if matches!(self.focus, Stage::Prs | Stage::Builds) && self.focused_error().is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        match self.focus {
            Stage::Prs => {
                let pr = self.prs.selected().ok_or("nothing selected")?;
                Ok(Menu {
                    title: format!("Manage #{}", pr.number),
                    offers: self.pr_offers(pr),
                })
            }
            Stage::Builds => {
                let build = self.builds.selected().ok_or("nothing selected")?;
                Ok(Menu {
                    title: format!("Manage run {}", build.run_number),
                    offers: vec![self.rerun_offer()],
                })
            }
            Stage::Queue | Stage::Deployed => Err("S manages My PRs and Main builds".to_string()),
        }
    }

    fn rerun_offer(&self) -> Offer {
        let target = self.rerun_target();
        let label = match &target {
            Ok(rerun) if rerun.run_ids.len() > 1 => {
                format!("rerun failed jobs ({} runs)", rerun.run_ids.len())
            }
            _ => "rerun failed jobs".to_string(),
        };
        offer('f', &label, target.map(Choice::Rerun))
    }

    fn pr_offers(&self, pr: &PullRequest) -> Vec<Offer> {
        let status = |change| {
            Ok(Choice::Status(StatusTarget {
                number: pr.number,
                repo: pr.repo.clone(),
                title: pr.title.clone(),
                head_oid: pr.head_oid.clone(),
                change,
            }))
        };
        let update = match self.config.prs.update_method {
            UpdateMethod::Merge => "update branch",
            UpdateMethod::Rebase => "rebase branch",
        };
        let queued = pr.queue_position.is_some();
        let draft = if pr.is_draft {
            offer('t', "mark ready", status(StatusChange::Ready))
        } else if queued {
            offer(
                't',
                "convert to draft",
                Err(format!("#{} is in the merge queue", pr.number)),
            )
        } else if let Some((number, _)) = self.merge_landing {
            offer(
                't',
                "convert to draft",
                Err(format!("waiting for My PRs to catch up with #{number}")),
            )
        } else {
            offer('t', "convert to draft", status(StatusChange::Draft))
        };
        let auto = if pr.can_disable_auto_merge {
            offer(
                'a',
                "disable auto-merge",
                status(StatusChange::DisableAutoMerge {
                    node_id: pr.node_id.clone(),
                }),
            )
        } else if pr.can_enable_auto_merge {
            offer(
                'a',
                "enable auto-merge",
                status(StatusChange::EnableAutoMerge {
                    node_id: pr.node_id.clone(),
                    method: self.config.prs.merge_method,
                    head_oid: pr.head_oid.clone(),
                }),
            )
        } else {
            offer(
                'a',
                "enable auto-merge",
                Err(format!("GitHub does not allow it for #{}", pr.number)),
            )
        };
        let dequeue = if queued {
            status(StatusChange::Dequeue {
                node_id: pr.node_id.clone(),
            })
        } else {
            Err(format!("#{} is not queued", pr.number))
        };
        let merge = if pr.merge_queue_enabled {
            offer(
                'm',
                "add to merge queue",
                self.merge_target().and_then(|_| {
                    status(StatusChange::Enqueue {
                        node_id: pr.node_id.clone(),
                        head_oid: pr.head_oid.clone(),
                    })
                }),
            )
        } else {
            offer('m', "merge", self.merge_target().map(Choice::Merge))
        };
        vec![
            merge,
            offer('u', update, self.update_target().map(Choice::Update)),
            self.rerun_offer(),
            draft,
            auto,
            offer('e', "leave the merge queue", dequeue),
            offer('x', "close", status(StatusChange::Close)),
        ]
    }

    fn manage(&mut self, key: KeyEvent) -> Action {
        let Mode::Manage(menu) = std::mem::take(&mut self.mode) else {
            return Action::Continue;
        };
        let picked = match key.code {
            KeyCode::Char(letter) if key.modifiers.is_empty() => menu
                .offers
                .iter()
                .find(|offer| offer.key == letter)
                .cloned(),
            _ => None,
        };
        let Some(picked) = picked else {
            self.notice = Some("nothing changed".to_string());
            return Action::Continue;
        };
        let Ok(choice) = picked.choice.clone() else {
            self.mode = Mode::Manage(menu);
            return Action::Continue;
        };
        match self.menu() {
            Err(message) => {
                self.notice = Some(message);
                return Action::Continue;
            }
            Ok(fresh) if fresh.title != menu.title => {
                self.notice = Some(changed(self.focus));
                return Action::Continue;
            }
            Ok(fresh) => match fresh
                .offers
                .into_iter()
                .find(|offer| offer.key == picked.key)
            {
                Some(offer) if offer == picked => {}
                Some(Offer {
                    choice: Err(reason),
                    ..
                }) => {
                    self.notice = Some(reason);
                    return Action::Continue;
                }
                _ => {
                    self.notice = Some(changed(self.focus));
                    return Action::Continue;
                }
            },
        }
        match choice {
            Choice::Merge(target) => self.mode = Mode::Confirm(Pending::Merge(target)),
            Choice::Update(target) if target.method == UpdateMethod::Rebase => {
                self.mode = Mode::Confirm(Pending::UpdateBranch(target));
            }
            Choice::Update(target) => return self.send_update(target),
            Choice::Rerun(rerun) => return self.send_rerun(rerun),
            Choice::Status(target)
                if target.change == StatusChange::Close || target.change.lands_a_merge() =>
            {
                self.mode = Mode::Confirm(Pending::Status(target));
            }
            Choice::Status(target) => return self.send_status(target),
        }
        Action::Continue
    }

    fn confirm_status(&mut self, key: KeyEvent, target: StatusTarget) -> Action {
        if key.code != KeyCode::Char('y') || !key.modifiers.is_empty() {
            self.notice = Some("nothing changed".to_string());
            return Action::Continue;
        }
        let letter = match target.change {
            StatusChange::EnableAutoMerge { .. } => 'a',
            StatusChange::Enqueue { .. } => 'm',
            _ => 'x',
        };
        if let Err(message) = self.reconfirm(letter, target.number, Choice::Status(target.clone()))
        {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.send_status(target)
    }

    fn reconfirm(&self, key: char, number: u64, expected: Choice) -> Result<(), String> {
        let menu = self.menu()?;
        if menu.title != format!("Manage #{number}") {
            return Err(changed(Stage::Prs));
        }
        match menu.offers.into_iter().find(|offer| offer.key == key) {
            Some(Offer {
                choice: Ok(choice), ..
            }) if choice == expected => Ok(()),
            Some(Offer {
                choice: Err(reason),
                ..
            }) => Err(reason),
            _ => Err(changed(Stage::Prs)),
        }
    }

    fn send_status(&mut self, target: StatusTarget) -> Action {
        self.notice = Some(progress(target.number, &target.change));
        self.changing = Some((target.number, target.change.clone()));
        Action::ChangeStatus(target)
    }

    pub fn receive_status(
        &mut self,
        number: u64,
        change: &StatusChange,
        result: &Result<(), String>,
    ) -> bool {
        self.changing = None;
        self.notice = Some(match result {
            Ok(()) => done(number, change),
            Err(message) => format!("#{number}: {message}"),
        });
        if result.is_ok() && change.lands_a_merge() {
            self.merge_landing = Some((number, false));
        }
        result.is_ok()
    }

    fn send_rerun(&mut self, rerun: Rerun) -> Action {
        self.notice = Some(format!("rerunning {}…", rerun.label));
        self.rerunning = Some((self.focus, rerun.label));
        Action::Rerun {
            repo: rerun.repo,
            run_ids: rerun.run_ids,
        }
    }

    pub fn receive_rerun(&mut self, results: Vec<(u64, Result<(), String>)>) -> Option<Stage> {
        let (stage, label) = self.rerunning.take()?;
        let errors: Vec<String> = results
            .iter()
            .filter_map(|(run_id, result)| {
                result
                    .as_ref()
                    .err()
                    .map(|message| format!("run {run_id}: {message}"))
            })
            .collect();
        let reran = results.len() - errors.len();
        self.notice = Some(match (reran, errors.is_empty()) {
            (_, true) => format!("rerun requested: {label}"),
            (0, false) => format!("rerun failed: {}", errors.join("; ")),
            _ => format!(
                "rerun requested for {reran} of {} runs; {}",
                results.len(),
                errors.join("; ")
            ),
        });
        (reran > 0).then_some(stage)
    }

    fn confirm_merge(&mut self, key: KeyEvent, target: MergeTarget) -> Action {
        if key.code != KeyCode::Char('y') || !key.modifiers.is_empty() {
            self.notice = Some("merge cancelled".to_string());
            return Action::Continue;
        }
        if let Err(message) = self.reconfirm('m', target.number, Choice::Merge(target.clone())) {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.merging = Some(target.number);
        self.notice = Some(format!("merging #{}…", target.number));
        Action::Merge(target)
    }

    fn merge_target(&self) -> Result<MergeTarget, String> {
        let pr = self.prs.selected().ok_or("nothing selected")?;
        if pr.is_draft {
            return Err(format!("#{} is a draft", pr.number));
        }
        if pr.queue_position.is_some() {
            return Err(format!("#{} is already queued", pr.number));
        }
        Ok(MergeTarget {
            number: pr.number,
            repo: pr.repo.clone(),
            title: pr.title.clone(),
            head_oid: pr.head_oid.clone(),
        })
    }

    pub fn receive_merged(&mut self, number: u64, result: &Result<(), String>) {
        self.merging = None;
        self.merge_landing = result.is_ok().then_some((number, false));
        self.notice = Some(match result {
            Ok(()) => format!("merge sent for #{number}"),
            Err(message) => format!("#{number}: {message}"),
        });
    }

    fn confirm_update(&mut self, key: KeyEvent, target: UpdateTarget) -> Action {
        if key.code != KeyCode::Char('y') || !key.modifiers.is_empty() {
            self.notice = Some("update cancelled".to_string());
            return Action::Continue;
        }
        if let Err(message) = self.reconfirm('u', target.number, Choice::Update(target.clone())) {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.send_update(target)
    }

    fn send_update(&mut self, target: UpdateTarget) -> Action {
        self.updating = Some(target.number);
        self.notice = Some(format!("updating #{}…", target.number));
        Action::UpdateBranch(target)
    }

    fn update_target(&self) -> Result<UpdateTarget, String> {
        if self.focused_error().is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        let pr = self.prs.selected().ok_or("nothing selected")?;
        if let Some((number, _)) = self.merge_landing {
            return Err(format!("waiting for My PRs to catch up with #{number}"));
        }
        if pr.queue_position.is_some() {
            return Err(format!(
                "#{} is in the merge queue; updating it would drop it",
                pr.number
            ));
        }
        if pr.merge_state == MergeState::Dirty {
            return Err(conflicts(pr.number));
        }
        Ok(UpdateTarget {
            number: pr.number,
            repo: pr.repo.clone(),
            title: pr.title.clone(),
            head_oid: pr.head_oid.clone(),
            method: self.config.prs.update_method,
        })
    }

    fn follow_up_prs(&mut self) {
        self.follow_up = Some(self.now + FOLLOW_UP);
    }

    pub fn follow_up_due(&mut self) -> bool {
        let due = self.follow_up.is_some_and(|at| self.now >= at);
        if due {
            self.follow_up = None;
        }
        due
    }

    pub fn receive_branch_update(
        &mut self,
        number: u64,
        result: &Result<BranchUpdate, String>,
    ) -> bool {
        self.updating = None;
        self.notice = Some(match result {
            Ok(BranchUpdate::Updated) => format!("branch of #{number} updated"),
            Ok(BranchUpdate::UpToDate) => format!("#{number} is already up to date"),
            Ok(BranchUpdate::Conflicts) => conflicts(number),
            Err(message) => format!("update failed for #{number}: {message}"),
        });
        matches!(result, Ok(BranchUpdate::Updated))
    }

    fn rerun_target(&self) -> Result<Rerun, String> {
        if matches!(self.focus, Stage::Builds | Stage::Prs) && self.focused_error().is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        match self.focus {
            Stage::Builds => {
                let build = self.builds.selected().ok_or("nothing selected")?;
                let label = format!("run {}", build.run_number);
                let newest = self.builds.all().iter().map(|b| b.run_number).max();
                if newest != Some(build.run_number) {
                    return Err("only the newest main build can be rerun".to_string());
                }
                if !build.is_settled() {
                    return Err(format!("{label} is still running"));
                }
                if !matches!(build.status, BuildStatus::Failure | BuildStatus::Cancelled) {
                    return Err("nothing failed to rerun".to_string());
                }
                let repo = self.builds_repo.clone().ok_or("no repository known")?;
                Ok(Rerun {
                    repo,
                    run_ids: vec![build.id],
                    label,
                })
            }
            Stage::Prs => {
                let pr = self.prs.selected().ok_or("nothing selected")?;
                let run_ids = pr.rerunnable_runs();
                let label = match run_ids.len() {
                    0 => return Err("no finished failing run to rerun".to_string()),
                    1 => format!("#{}", pr.number),
                    runs => format!("#{} ({runs} runs)", pr.number),
                };
                Ok(Rerun {
                    repo: pr.repo.clone(),
                    run_ids,
                    label,
                })
            }
            Stage::Queue => Err("nothing to rerun in the merge queue".to_string()),
            Stage::Deployed => Err("nothing to rerun here".to_string()),
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char(c) => self.edit_filter(|q| q.push(c)),
            KeyCode::Backspace => self.edit_filter(|q| {
                q.pop();
            }),
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                if self.query.as_deref() == Some("") {
                    self.query = None;
                }
                self.sync_filter();
                Action::Continue
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.clear_filter();
                Action::Continue
            }
            _ => self.handle_normal_key(key),
        }
    }

    fn edit_filter(&mut self, edit: impl FnOnce(&mut String)) -> Action {
        if let Mode::Filter(query) = &mut self.mode {
            edit(query);
            self.query = Some(query.clone());
        }
        self.sync_filter();
        Action::Continue
    }

    fn clear_filter(&mut self) {
        self.query = None;
        self.sync_filter();
    }

    /// `Esc` closes the topmost thing, one per press: the details pane, then zoom, then a
    /// live filter. With nothing open it does nothing; it never quits.
    fn escape(&mut self) {
        if self.details {
            self.details = false;
            self.reset_scroll();
        } else if self.zoom {
            self.zoom = false;
        } else if self.query.is_some() {
            self.clear_filter();
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Action {
        let pending_g = std::mem::take(&mut self.pending_g);
        match key.code {
            KeyCode::Char('q') => return Action::Quit,
            KeyCode::Esc => self.escape(),
            KeyCode::Char(digit @ '1'..='4') => {
                let index = digit as usize - '1' as usize;
                self.refocus(|_| Stage::ALL[index]);
            }
            KeyCode::Char('/') => {
                let current = self.query.clone().unwrap_or_default();
                self.query = Some(current.clone());
                self.mode = Mode::Filter(current);
                self.sync_filter();
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Char('c') => {
                self.changelog_scroll = 0;
                self.mode = Mode::Changelog;
            }
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
                self.reset_scroll();
            }
            KeyCode::Char('L') => return self.toggle_log(),
            KeyCode::Char('S') => self.ask_to_manage(),
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
            Input::Tick(now) => {
                app.now = now;
                if app.follow_up_due() {
                    request(Request::Refresh(Stage::Prs));
                }
            }
            Input::Fetching(stage) => app.fetching(stage),
            Input::Data(stage, data) => app.receive(stage, data),
            Input::Jobs(run_id, result) => app.receive_jobs(run_id, result),
            Input::Log(job_id, result) => app.receive_log(job_id, result),
            Input::BranchUpdated(number, result) => {
                if app.receive_branch_update(number, &result) {
                    request(Request::Refresh(Stage::Prs));
                    app.follow_up_prs();
                }
            }
            Input::Merged(number, result) => {
                app.receive_merged(number, &result);
                if result.is_ok() {
                    request(Request::Refresh(Stage::Prs));
                    request(Request::Refresh(Stage::Queue));
                }
            }
            Input::StatusChanged(number, change, result) => {
                if app.receive_status(number, &change, &result) {
                    request(Request::Refresh(Stage::Prs));
                    if change != StatusChange::Ready {
                        request(Request::Refresh(Stage::Queue));
                    }
                    app.follow_up_prs();
                }
            }
            Input::Rerun(result) => {
                if let Some(stage) = app.receive_rerun(result) {
                    request(Request::Refresh(stage));
                    if stage == Stage::Prs {
                        app.follow_up_prs();
                    }
                }
            }
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
                Action::Log { run_id, job_id } => request(Request::Log { run_id, job_id }),
                Action::Merge(target) => request(Request::Merge(target)),
                Action::UpdateBranch(target) => request(Request::UpdateBranch(target)),
                Action::Rerun { repo, run_ids } => request(Request::Rerun { repo, run_ids }),
                Action::ChangeStatus(target) => request(Request::ChangeStatus(target)),
                Action::Continue => {}
            },
        }
        if let Some(run_id) = app.jobs_needed() {
            request(Request::Jobs { run_id });
        }
    }
}

fn offer(key: char, label: &str, choice: Result<Choice, String>) -> Offer {
    Offer {
        key,
        label: label.to_string(),
        choice,
    }
}

fn changed(stage: Stage) -> String {
    match stage {
        Stage::Builds => "the run changed; press S again".to_string(),
        _ => "the pull request changed; press S again".to_string(),
    }
}

fn progress(number: u64, change: &StatusChange) -> String {
    match change {
        StatusChange::Ready => format!("marking #{number} ready…"),
        StatusChange::Draft => format!("converting #{number} to draft…"),
        StatusChange::EnableAutoMerge { .. } => format!("enabling auto-merge for #{number}…"),
        StatusChange::DisableAutoMerge { .. } => format!("disabling auto-merge for #{number}…"),
        StatusChange::Dequeue { .. } => format!("taking #{number} out of the merge queue…"),
        StatusChange::Enqueue { .. } => format!("adding #{number} to the merge queue…"),
        StatusChange::Close => format!("closing #{number}…"),
    }
}

fn done(number: u64, change: &StatusChange) -> String {
    match change {
        StatusChange::Ready => format!("#{number} marked ready"),
        StatusChange::Draft => format!("#{number} converted to draft"),
        StatusChange::EnableAutoMerge { .. } => format!("auto-merge enabled for #{number}"),
        StatusChange::DisableAutoMerge { .. } => format!("auto-merge disabled for #{number}"),
        StatusChange::Dequeue { .. } => format!("#{number} left the merge queue"),
        StatusChange::Enqueue { .. } => format!("#{number} added to the merge queue"),
        StatusChange::Close => format!("#{number} closed"),
    }
}

fn conflicts(number: u64) -> String {
    format!("#{number} has conflicts with its base; resolve them locally")
}
