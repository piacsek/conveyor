use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App, Stage};

use super::{Choice, WriteKind, may_have_happened};

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
        self.start_write(target.number, WriteKind::Merge);
        Action::Merge(target)
    }

    pub fn receive_merged(
        &mut self,
        number: u64,
        result: &Result<(), crate::sources::failure::Failure>,
    ) -> bool {
        let write = self.finish_write(number, &WriteKind::Merge);
        if let Err(message) = result {
            self.fail(write, message);
        }
        self.notice = Some(match result {
            Ok(()) => format!("merge sent for #{number}"),
            Err(message) => format!("#{number}: {message}"),
        });
        let landed = may_have_happened(result);
        if landed {
            self.land(number);
        }
        landed
    }

    pub(super) fn land(&mut self, number: u64) {
        let tab = self.stage(Stage::Prs).active_tab();
        self.merge_landing
            .retain(|(landing, _, _)| *landing != number);
        self.merge_landing.push((number, tab, false));
    }
}
