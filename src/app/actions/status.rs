use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App, pull_key};
use crate::model::prs::StatusChange;

use super::{Choice, WriteKind, may_have_happened};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub change: StatusChange,
}

impl StatusTarget {
    pub fn key(&self) -> u64 {
        pull_key(&self.repo, self.number)
    }
}

impl App {
    pub(super) fn confirm_status(&mut self, key: KeyEvent, target: StatusTarget) -> Action {
        if key.code != KeyCode::Char('y') || !key.modifiers.is_empty() {
            self.notice = Some("nothing changed".to_string());
            return Action::Continue;
        }
        let letter = match target.change {
            StatusChange::EnableAutoMerge { .. } => 'a',
            StatusChange::Enqueue { .. } => 'm',
            _ => 'x',
        };
        if let Err(message) = self.reconfirm(
            letter,
            &target.repo,
            target.number,
            Choice::Status(target.clone()),
        ) {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.send_status(target)
    }

    pub(super) fn send_status(&mut self, target: StatusTarget) -> Action {
        self.start_write(
            target.key(),
            WriteKind::Status(target.number, target.change.clone()),
        );
        Action::ChangeStatus(target)
    }

    pub fn receive_status(
        &mut self,
        target: &StatusTarget,
        result: &Result<(), crate::sources::failure::Failure>,
    ) -> bool {
        let (number, change) = (target.number, &target.change);
        let write = self.finish_write(target.key(), &WriteKind::Status(number, change.clone()));
        if let Err(message) = result {
            self.fail(write, message);
        }
        self.notice = Some(match result {
            Ok(()) => done(number, change),
            Err(message) => format!("#{number}: {message}"),
        });
        let happened = may_have_happened(result);
        if happened && change.lands_a_merge() {
            self.land(target.key());
        }
        happened
    }
}

pub(super) fn progress(number: u64, change: &StatusChange) -> String {
    match change {
        StatusChange::Ready => format!("marking #{number} ready…"),
        StatusChange::Draft => format!("converting #{number} to draft…"),
        StatusChange::EnableAutoMerge { .. } => format!("enabling auto-merge for #{number}…"),
        StatusChange::DisableAutoMerge { .. } => format!("disabling auto-merge for #{number}…"),
        StatusChange::Dequeue { .. } => format!("taking #{number} out of the merge queue…"),
        StatusChange::Enqueue { .. } => format!("adding #{number} to the merge queue…"),
        StatusChange::Close => format!("closing #{number}…"),
        StatusChange::Approve => unimplemented!(),
    }
}

fn done(number: u64, change: &StatusChange) -> String {
    match change {
        StatusChange::Ready => format!("#{number} marked ready"),
        StatusChange::Draft => format!("#{number} converted to draft"),
        StatusChange::EnableAutoMerge { .. } => format!("auto-merge enabled for #{number}"),
        StatusChange::DisableAutoMerge { .. } => format!("auto-merge disabled for #{number}"),
        StatusChange::Dequeue { .. } => format!("#{number} left the merge queue"),
        StatusChange::Enqueue { .. } => format!("#{number} added to the merge queue"),
        StatusChange::Close => format!("#{number} closed"),
        StatusChange::Approve => unimplemented!(),
    }
}
