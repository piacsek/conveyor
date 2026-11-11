use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App};

use super::Choice;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
}

impl App {
    pub(super) fn confirm_merge(&mut self, key: KeyEvent, target: MergeTarget) -> Action {
        if key.code != KeyCode::Char('y') || !key.modifiers.is_empty() {
            self.notice = Some("merge cancelled".to_string());
            return Action::Continue;
        }
        if let Err(message) = self.reconfirm('m', target.number, Choice::Merge(target.clone())) {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.merging = Some(target.number);
        self.notice = Some(format!("merging #{}…", target.number));
        Action::Merge(target)
    }

    pub fn receive_merged(&mut self, number: u64, result: &Result<(), String>) {
        self.merging = None;
        self.merge_landing = result.is_ok().then_some((number, false));
        self.notice = Some(match result {
            Ok(()) => format!("merge sent for #{number}"),
            Err(message) => format!("#{number}: {message}"),
        });
    }
}
