use std::time::SystemTime;

use crate::config::Config;
use crate::model::builds::{Build, Builds};
use crate::model::deployed::{Deployed, Deployment};
use crate::model::prs::PullRequest;
use crate::model::queue::Queue;

use super::{App, Listing, Menu};

mod builds;
mod deployed;
mod prs;
mod queue;

pub use builds::BuildsStage;
pub use deployed::DeployedStage;
pub use prs::PrsStage;
pub use queue::QueueStage;

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

impl Rows {
    pub fn stage(&self) -> Stage {
        match self {
            Rows::Prs(_) => Stage::Prs,
            Rows::Queue(_) => Stage::Queue,
            Rows::Builds(_) => Stage::Builds,
            Rows::Deployed(_) => Stage::Deployed,
        }
    }
}

pub struct Context<'a> {
    pub config: &'a Config,
    pub builds: &'a BuildsStage,
    pub merge_landing: &'a [(u64, bool)],
}

impl Context<'_> {
    pub fn main_build(&self, sha: &str) -> Option<&Build> {
        self.builds
            .column
            .all()
            .iter()
            .find(|build| build.sha == sha)
    }

    pub fn landing(&self, number: u64) -> bool {
        self.merge_landing
            .iter()
            .any(|(landing, _)| *landing == number)
    }

    pub fn behind_main(&self, row: &Deployment) -> Option<usize> {
        let sha = row.sha.as_ref()?;
        self.builds
            .column
            .all()
            .iter()
            .position(|build| &build.sha == sha)
    }

    /// The repository the Deployed column watches: the configured one with a `[[repo.deploy]]`,
    /// else the one the main builds came from.
    pub fn deploy_repo(&self) -> Option<String> {
        self.config
            .repo
            .iter()
            .find(|repo| !repo.deploy.is_empty())
            .map(|repo| repo.name.clone())
            .or_else(|| self.builds.repo.clone())
    }
}

pub trait StageView {
    fn listing(&self) -> &dyn Listing;

    fn listing_mut(&mut self) -> &mut dyn Listing;

    fn views(&self) -> usize {
        1
    }

    fn view_mut(&mut self, _: usize) -> &mut dyn Listing {
        self.listing_mut()
    }

    fn receive(&mut self, view: usize, rows: Rows, now: SystemTime);

    fn error(&self) -> Option<&str> {
        self.listing().error()
    }

    fn pull_target(&self) -> Result<(u64, String), String>;

    fn build_url(&self, context: &Context) -> Result<String, String>;

    fn selected_run(&self) -> Option<&Build> {
        None
    }

    fn runs(&self) -> Vec<&Build> {
        Vec::new()
    }

    fn menu(&self, _: &Context) -> Result<Menu, String> {
        Err("S manages My PRs and Main builds".to_string())
    }

    fn changed(&self) -> String {
        "the pull request changed; press S again".to_string()
    }
}

impl App {
    pub fn stage(&self, stage: Stage) -> &dyn StageView {
        match stage {
            Stage::Prs => &self.prs,
            Stage::Queue => &self.queue,
            Stage::Builds => &self.builds,
            Stage::Deployed => &self.deployed,
        }
    }

    pub fn stage_mut(&mut self, stage: Stage) -> &mut dyn StageView {
        match stage {
            Stage::Prs => &mut self.prs,
            Stage::Queue => &mut self.queue,
            Stage::Builds => &mut self.builds,
            Stage::Deployed => &mut self.deployed,
        }
    }

    pub fn context(&self) -> Context<'_> {
        Context {
            config: &self.config,
            builds: &self.builds,
            merge_landing: &self.merge_landing,
        }
    }

    pub fn receive(&mut self, stage: Stage, view: usize, data: Result<Rows, String>) {
        if view >= self.stage(stage).views() {
            return;
        }
        if stage == Stage::Prs {
            self.merge_landing.retain(|(_, started)| !started);
        }
        let selected = self.selected_key();
        self.receive_rows(stage, view, data);
        if self.selected_key() != selected {
            self.reset_scroll();
        }
    }

    pub(super) fn selected_key(&self) -> Option<u64> {
        self.stage(self.focus).listing().selected_key()
    }

    fn receive_rows(&mut self, stage: Stage, view: usize, data: Result<Rows, String>) {
        match data {
            Ok(rows) if rows.stage() == stage => {
                let now = self.now;
                self.stage_mut(stage).receive(view, rows, now);
                self.age_jobs(stage);
                self.prune_failures(stage);
            }
            Ok(_) => {}
            Err(message) => self.stage_mut(stage).view_mut(view).fail(message),
        }
    }

    pub fn fetching(&mut self, stage: Stage, view: usize) {
        if view >= self.stage(stage).views() {
            return;
        }
        if stage == Stage::Prs {
            for (_, started) in &mut self.merge_landing {
                *started = true;
            }
        }
        self.stage_mut(stage).view_mut(view).set_refreshing(true);
    }

    pub fn any_refreshing(&self) -> bool {
        Stage::ALL
            .into_iter()
            .any(|stage| self.stage(stage).listing().is_refreshing())
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
}
