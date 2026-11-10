use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub const OPEN_DEBOUNCE: Duration = Duration::from_secs(1);
pub const FOLLOW_UP: Duration = Duration::from_secs(10);

use ratatui::crossterm::event::KeyEvent;

use crate::config::Config;
use crate::model::builds::{Build, Builds};
use crate::model::deployed::{Deployed, Deployment};
use crate::model::jobs::Job;
use crate::model::prs::{BranchUpdate, PullRequest, StatusChange};
use crate::model::queue::{Queue, QueueEntry};

mod column;
mod jobs;
mod keys;
mod manage;
mod run;

pub use column::{Column, ColumnState, Navigable, Order, Row};
pub use jobs::{JobsState, LogState};
pub use manage::{
    CheckoutTarget, Choice, Menu, MergeTarget, Offer, Pending, Rerun, StatusTarget, UpdateTarget,
};
pub use run::run;

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
    CheckedOut(CheckoutTarget, Result<(), String>),
    Tick(SystemTime),
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
    Checkout(CheckoutTarget),
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
    Checkout(CheckoutTarget),
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
    checking_out: Option<u64>,
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
            checking_out: None,
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
