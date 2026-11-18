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
    Merge(u64),
    Update(u64),
    Status(u64, StatusChange),
    Checkout(u64),
    Rerun {
        run_ids: Vec<u64>,
        label: String,
        tab: usize,
    },
}

impl Write {
    pub fn progress(&self) -> String {
        match &self.kind {
            WriteKind::Merge(number) => format!("merging #{number}…"),
            WriteKind::Update(number) => format!("updating #{number}…"),
            WriteKind::Status(number, change) => progress(*number, change),
            WriteKind::Checkout(number) => format!("checking out #{number}…"),
            WriteKind::Rerun { label, .. } => format!("rerunning {label}…"),
        }
    }

    pub fn action(&self) -> String {
        match &self.kind {
            WriteKind::Merge(_) => "merge",
            WriteKind::Update(_) => "update",
            WriteKind::Status(_, change) => match change {
                StatusChange::Ready => "mark ready",
                StatusChange::Draft => "convert to draft",
                StatusChange::EnableAutoMerge { .. } => "enable auto-merge",
                StatusChange::DisableAutoMerge { .. } => "disable auto-merge",
                StatusChange::Dequeue { .. } => "leave the merge queue",
                StatusChange::Enqueue { .. } => "add to the merge queue",
                StatusChange::Close => "close",
                StatusChange::Approve => unimplemented!(),
            },
            WriteKind::Checkout(_) => "check out",
            WriteKind::Rerun { .. } => "rerun",
        }
        .to_string()
    }

    pub fn word(&self) -> String {
        match &self.kind {
            WriteKind::Merge(_) => "merging…",
            WriteKind::Update(_) => "updating…",
            WriteKind::Status(_, change) => match change {
                StatusChange::Ready => "marking ready…",
                StatusChange::Draft => "converting to draft…",
                StatusChange::EnableAutoMerge { .. } => "enabling auto-merge…",
                StatusChange::DisableAutoMerge { .. } => "disabling auto-merge…",
                StatusChange::Dequeue { .. } => "leaving the merge queue…",
                StatusChange::Enqueue { .. } => "adding to the merge queue…",
                StatusChange::Close => "closing…",
                StatusChange::Approve => unimplemented!(),
            },
            WriteKind::Checkout(_) => "checking out…",
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
