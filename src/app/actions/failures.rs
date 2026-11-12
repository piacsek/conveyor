use std::time::SystemTime;

use crate::app::{App, Stage};

use super::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub stage: Stage,
    pub subject: u64,
    pub action: String,
    pub message: String,
    pub at: SystemTime,
}

impl Failure {
    pub fn line(&self) -> String {
        format!("! {} failed: {}", self.action, self.message)
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
            .count()
    }

    pub fn selected_failure(&self) -> Option<&Failure> {
        self.failure_on(self.focus, self.selected_key()?)
    }

    pub(in crate::app) fn clear_failure(&mut self, stage: Stage, subject: u64) {
        self.failures
            .retain(|failure| failure.stage != stage || failure.subject != subject);
    }

    pub(super) fn fail(&mut self, write: Option<Write>, message: &str) {
        let Some(write) = write else {
            return;
        };
        self.failures.push(Failure {
            stage: write.stage,
            subject: write.subject,
            action: write.action(),
            message: message.to_string(),
            at: self.now,
        });
    }
}
