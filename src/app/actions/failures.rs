use std::time::SystemTime;

use crate::app::{App, Stage};
use crate::sources::failure::FailureKind;

use super::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub stage: Stage,
    pub subject: u64,
    pub action: String,
    pub kind: FailureKind,
    pub message: String,
    pub at: SystemTime,
}

impl Failure {
    pub fn line(&self) -> String {
        let message = self
            .message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if self.kind == FailureKind::TimedOut {
            return format!("! {message}");
        }
        format!("! {} failed: {message}", self.action)
    }
}

impl App {
    pub fn failure_on(&self, stage: Stage, subject: u64) -> Option<&Failure> {
        self.failures
            .iter()
            .find(|failure| failure.stage == stage && failure.subject == subject)
    }

    pub fn failure_count(&self, stage: Stage) -> usize {
        self.failures
            .iter()
            .filter(|failure| failure.stage == stage)
            .filter(|failure| self.stage(stage).listing().holds(failure.subject))
            .count()
    }

    pub fn selected_failure(&self) -> Option<&Failure> {
        self.failure_on(self.focus, self.selected_key()?)
    }

    pub(in crate::app) fn prune_failures(&mut self, stage: Stage) {
        let failures = std::mem::take(&mut self.failures);
        let view = self.stage(stage);
        self.failures = failures
            .into_iter()
            .filter(|failure| failure.stage != stage || view.holds(failure.subject))
            .collect();
    }

    pub(in crate::app) fn clear_failure(&mut self, stage: Stage, subject: u64) {
        self.failures
            .retain(|failure| failure.stage != stage || failure.subject != subject);
    }

    pub(super) fn fail(&mut self, write: Option<Write>, cause: &crate::sources::failure::Failure) {
        let Some(write) = write else {
            return;
        };
        if !self.stage(write.stage).holds(write.subject) {
            return;
        }
        self.failures.push(Failure {
            stage: write.stage,
            subject: write.subject,
            action: write.action(),
            kind: cause.kind(),
            message: cause.to_string(),
            at: self.now,
        });
    }
}

pub(super) fn timed_out<T>(result: &Result<T, crate::sources::failure::Failure>) -> bool {
    result
        .as_ref()
        .is_err_and(|failure| failure.kind() == FailureKind::TimedOut)
}

pub(super) fn may_have_happened(result: &Result<(), crate::sources::failure::Failure>) -> bool {
    result.is_ok() || timed_out(result)
}
