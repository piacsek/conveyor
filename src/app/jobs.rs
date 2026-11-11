use crate::model::builds::{Build, BuildStatus};
use crate::model::jobs::Job;

use super::{Action, App, Stage};

const EAGER_JOBS: usize = 5;

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

impl App {
    pub(super) fn age_jobs(&mut self, stage: Stage) {
        let runs: Vec<(u64, bool, u64)> = match stage {
            Stage::Queue => self
                .queue
                .column
                .all()
                .iter()
                .filter_map(|entry| entry.run.as_ref())
                .map(|run| (run.id, run.is_settled(), run.attempt))
                .collect(),
            Stage::Builds => self
                .builds
                .column
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
            Stage::Builds => self.builds.column.selected(),
            Stage::Queue => self.queue.column.selected()?.run.as_ref(),
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
            .column
            .all()
            .iter()
            .chain(
                self.queue
                    .column
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
            .column
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
    pub(super) fn toggle_log(&mut self) -> Action {
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
}
