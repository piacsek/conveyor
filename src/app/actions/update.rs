use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App, pull_key};
use crate::config::UpdateMethod;
use crate::model::prs::BranchUpdate;

use super::{Choice, WriteKind, timed_out};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub method: UpdateMethod,
}

impl UpdateTarget {
    pub fn key(&self) -> u64 {
        pull_key(&self.repo, self.number)
    }
}

impl App {
    pub(super) fn confirm_update(&mut self, key: KeyEvent, target: UpdateTarget) -> Action {
        if key.code != KeyCode::Char('y') || !key.modifiers.is_empty() {
            self.notice = Some("update cancelled".to_string());
            return Action::Continue;
        }
        if let Err(message) = self.reconfirm(
            'u',
            &target.repo,
            target.number,
            Choice::Update(target.clone()),
        ) {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.send_update(target)
    }

    pub(super) fn send_update(&mut self, target: UpdateTarget) -> Action {
        self.start_write(target.key(), WriteKind::Update(target.number));
        Action::UpdateBranch(target)
    }

    pub fn receive_branch_update(
        &mut self,
        target: &UpdateTarget,
        result: &Result<BranchUpdate, crate::sources::failure::Failure>,
    ) -> bool {
        let number = target.number;
        let write = self.finish_write(target.key(), &WriteKind::Update(number));
        match result {
            Ok(BranchUpdate::Conflicts) => self.fail(write, &CONFLICTS.to_string().into()),
            Err(message) => self.fail(write, message),
            Ok(_) => {}
        }
        self.notice = Some(match result {
            Ok(BranchUpdate::Updated) => format!("branch of #{number} updated"),
            Ok(BranchUpdate::UpToDate) => format!("#{number} is already up to date"),
            Ok(BranchUpdate::Conflicts) => conflicts(number),
            Err(message) => format!("update failed for #{number}: {message}"),
        });
        matches!(result, Ok(BranchUpdate::Updated)) || timed_out(result)
    }
}

const CONFLICTS: &str = "has conflicts with its base; resolve them locally";

pub(in crate::app) fn conflicts(number: u64) -> String {
    format!("#{number} {CONFLICTS}")
}
