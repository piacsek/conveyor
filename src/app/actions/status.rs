use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App};
use crate::model::prs::StatusChange;

use super::{Choice, WriteKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub change: StatusChange,
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
        if let Err(message) = self.reconfirm(letter, target.number, Choice::Status(target.clone()))
        {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.send_status(target)
    }

    pub(super) fn send_status(&mut self, target: StatusTarget) -> Action {
        self.start_write(target.number, WriteKind::Status(target.change.clone()));
        Action::ChangeStatus(target)
    }

    pub fn receive_status(
        &mut self,
        number: u64,
        change: &StatusChange,
        result: &Result<(), String>,
    ) -> bool {
        self.finish_write(number, &WriteKind::Status(change.clone()));
        self.notice = Some(match result {
            Ok(()) => done(number, change),
            Err(message) => format!("#{number}: {message}"),
        });
        if result.is_ok() && change.lands_a_merge() {
            self.land(number);
        }
        result.is_ok()
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
    }
}
