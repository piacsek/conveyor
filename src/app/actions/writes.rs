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
    Merge,
    Update,
    Status(StatusChange),
    Checkout,
    Rerun { run_ids: Vec<u64>, label: String },
}

impl Write {
    pub fn progress(&self) -> String {
        match &self.kind {
            WriteKind::Merge => format!("merging #{}…", self.subject),
            WriteKind::Update => format!("updating #{}…", self.subject),
            WriteKind::Status(change) => progress(self.subject, change),
            WriteKind::Checkout => format!("checking out #{}…", self.subject),
            WriteKind::Rerun { label, .. } => format!("rerunning {label}…"),
        }
    }

    pub fn action(&self) -> String {
        match &self.kind {
            WriteKind::Merge => "merge",
            WriteKind::Update => "update",
            WriteKind::Status(change) => match change {
                StatusChange::Ready => "mark ready",
                StatusChange::Draft => "convert to draft",
                StatusChange::EnableAutoMerge { .. } => "enable auto-merge",
                StatusChange::DisableAutoMerge { .. } => "disable auto-merge",
                StatusChange::Dequeue { .. } => "leave the merge queue",
                StatusChange::Enqueue { .. } => "add to the merge queue",
                StatusChange::Close => "close",
            },
            WriteKind::Checkout => "check out",
            WriteKind::Rerun { .. } => "rerun",
        }
        .to_string()
    }

    pub fn word(&self) -> String {
        match &self.kind {
            WriteKind::Merge => "merging…",
            WriteKind::Update => "updating…",
            WriteKind::Status(change) => match change {
                StatusChange::Ready => "marking ready…",
                StatusChange::Draft => "converting to draft…",
                StatusChange::EnableAutoMerge { .. } => "enabling auto-merge…",
                StatusChange::DisableAutoMerge { .. } => "disabling auto-merge…",
                StatusChange::Dequeue { .. } => "leaving the merge queue…",
                StatusChange::Enqueue { .. } => "adding to the merge queue…",
                StatusChange::Close => "closing…",
            },
            WriteKind::Checkout => "checking out…",
            WriteKind::Rerun { .. } => "rerunning…",
        }
        .to_string()
    }
}

impl App {
    pub fn write_on(&self, stage: Stage, subject: u64) -> Option<&Write> {
        self.writes
            .iter()
            .find(|write| write.stage == stage && write.subject == subject)
    }

    pub(super) fn start_write(&mut self, subject: u64, kind: WriteKind) {
        let write = Write {
            subject,
            stage: self.focus,
            kind,
        };
        self.notice = Some(write.progress());
        self.clear_failure(write.stage, write.subject);
        self.writes.push(write);
    }

    pub(super) fn finish_write(&mut self, subject: u64, kind: &WriteKind) -> Option<Write> {
        let index = self
            .writes
            .iter()
            .position(|write| write.subject == subject && &write.kind == kind)?;
        Some(self.writes.remove(index))
    }
}
