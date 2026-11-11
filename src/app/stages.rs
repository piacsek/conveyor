use crate::model::builds::Builds;
use crate::model::deployed::{Deployed, Deployment};
use crate::model::prs::PullRequest;
use crate::model::queue::Queue;

use super::{App, Row};

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

impl App {
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
