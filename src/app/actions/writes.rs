use crate::app::{App, Stage};
use crate::model::prs::StatusChange;

use super::status::progress;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    pub subject: u64,
    pub stage: Stage,
    pub kind: WriteKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteKind {
    Status(StatusChange),
}

impl Write {
    pub fn progress(&self) -> String {
        match &self.kind {
            WriteKind::Status(change) => progress(self.subject, change),
        }
    }
}

impl App {
    pub fn write_on(&self, stage: Stage, subject: u64) -> Option<&Write> {
        self.writes
            .iter()
            .find(|write| write.stage == stage && write.subject == subject)
    }

    pub(super) fn start_write(&mut self, subject: u64, kind: WriteKind) {
        self.writes.push(Write {
            subject,
            stage: self.focus,
            kind,
        });
    }

    pub(super) fn finish_write(&mut self, subject: u64, kind: &WriteKind) {
        if let Some(index) = self
            .writes
            .iter()
            .position(|write| write.subject == subject && &write.kind == kind)
        {
            self.writes.remove(index);
        }
    }
}
