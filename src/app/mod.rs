use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub const FOLLOW_UP: Duration = Duration::from_secs(10);

use ratatui::crossterm::event::KeyEvent;

use crate::config::Config;
use crate::model::deployed::Deployment;
use crate::model::jobs::Job;
use crate::model::prs::{BranchUpdate, StatusChange};

mod actions;
mod browser;
mod column;
mod jobs;
mod keymap;
mod navigation;
mod run;
mod stages;

pub use actions::{
    CheckoutTarget, Choice, Failure, Menu, MergeTarget, Offer, Pending, Rerun, StatusTarget,
    UpdateTarget, Write, WriteKind,
};
pub use browser::OPEN_DEBOUNCE;
pub use column::{Column, ColumnState, Listing, Navigable, Order, Row};
pub use jobs::{JobsState, LogState};
pub use run::run;
pub use stages::{
    BuildsStage, Context, DeployedStage, PrsStage, QueueStage, Rows, Stage, StageView,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(KeyEvent),
    Fetching(Stage, usize),
    Data(Stage, usize, Result<Rows, String>),
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
    Refresh(Stage, usize),
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
    pub prs: PrsStage,
    pub queue: QueueStage,
    pub builds: BuildsStage,
    pub deployed: DeployedStage,
    pub focus: Stage,
    pub mode: Mode,
    pub config: Config,
    pub now: SystemTime,
    pub utc_offset_secs: i32,
    pub notice: Option<String>,
    pub follow_up: Option<SystemTime>,
    pub merge_landing: Vec<(u64, bool)>,
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
    writes: Vec<Write>,
    failures: Vec<Failure>,
    pending_g: bool,
    last_open: Option<(String, SystemTime)>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self::at(SystemTime::now(), config)
    }

    pub fn at(now: SystemTime, config: Config) -> Self {
        Self {
            prs: PrsStage::new(config.prs.tabs().into_iter().map(|tab| tab.name).collect()),
            queue: QueueStage::default(),
            builds: BuildsStage::default(),
            deployed: DeployedStage::default(),
            focus: Stage::Prs,
            mode: Mode::Normal,
            config,
            now,
            utc_offset_secs: 0,
            notice: None,
            follow_up: None,
            merge_landing: Vec::new(),
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
            writes: Vec::new(),
            failures: Vec::new(),
            pending_g: false,
            last_open: None,
        }
    }

    pub fn behind_main(&self, row: &Deployment) -> Option<usize> {
        self.context().behind_main(row)
    }

    pub fn deploy_configured(&self) -> bool {
        self.config.repo.iter().any(|repo| !repo.deploy.is_empty())
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
