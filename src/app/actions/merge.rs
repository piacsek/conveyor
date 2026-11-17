use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App, Stage, pull_key};

use super::{Choice, WriteKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
}

impl MergeTarget {
    pub fn key(&self) -> u64 {
        pull_key(&self.repo, self.number)
    }
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
        self.start_write(target.key(), WriteKind::Merge(target.number));
        Action::Merge(target)
    }

    pub fn receive_merged(
        &mut self,
        target: &MergeTarget,
        result: &Result<(), crate::sources::failure::Failure>,
    ) {
        let number = target.number;
        let write = self.finish_write(target.key(), &WriteKind::Merge(number));
        match result {
            Ok(()) => self.land(target.key()),
            Err(message) => self.fail(write, message),
        }
        self.notice = Some(match result {
            Ok(()) => format!("merge sent for #{number}"),
            Err(message) => format!("#{number}: {message}"),
        });
    }

    pub(super) fn land(&mut self, key: u64) {
        let tab = self.stage(Stage::Prs).active_tab();
        self.merge_landing.retain(|(landing, _, _)| *landing != key);
        self.merge_landing.push((key, tab, false));
    }
}
