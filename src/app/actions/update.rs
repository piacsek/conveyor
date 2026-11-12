use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App};
use crate::config::UpdateMethod;
use crate::model::prs::BranchUpdate;

use super::{Choice, WriteKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub method: UpdateMethod,
}

impl App {
    pub(super) fn confirm_update(&mut self, key: KeyEvent, target: UpdateTarget) -> Action {
        if key.code != KeyCode::Char('y') || !key.modifiers.is_empty() {
            self.notice = Some("update cancelled".to_string());
            return Action::Continue;
        }
        if let Err(message) = self.reconfirm('u', target.number, Choice::Update(target.clone())) {
            self.notice = Some(message);
            return Action::Continue;
        }
        self.send_update(target)
    }

    pub(super) fn send_update(&mut self, target: UpdateTarget) -> Action {
        self.start_write(target.number, WriteKind::Update);
        Action::UpdateBranch(target)
    }

    pub fn receive_branch_update(
        &mut self,
        number: u64,
        result: &Result<BranchUpdate, String>,
    ) -> bool {
        self.finish_write(number, &WriteKind::Update);
        self.notice = Some(match result {
            Ok(BranchUpdate::Updated) => format!("branch of #{number} updated"),
            Ok(BranchUpdate::UpToDate) => format!("#{number} is already up to date"),
            Ok(BranchUpdate::Conflicts) => conflicts(number),
            Err(message) => format!("update failed for #{number}: {message}"),
        });
        matches!(result, Ok(BranchUpdate::Updated))
    }
}

pub(in crate::app) fn conflicts(number: u64) -> String {
    format!("#{number} has conflicts with its base; resolve them locally")
}
