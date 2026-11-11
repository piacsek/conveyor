use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{Action, App};
use crate::config::UpdateMethod;
use crate::model::prs::{BranchUpdate, MergeState};

use super::Choice;

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
        self.updating = Some(target.number);
        self.notice = Some(format!("updating #{}…", target.number));
        Action::UpdateBranch(target)
    }

    pub(super) fn update_target(&self) -> Result<UpdateTarget, String> {
        if self.focused_error().is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        let pr = self.prs.selected().ok_or("nothing selected")?;
        if let Some((number, _)) = self.merge_landing {
            return Err(format!("waiting for My PRs to catch up with #{number}"));
        }
        if pr.queue_position.is_some() {
            return Err(format!(
                "#{} is in the merge queue; updating it would drop it",
                pr.number
            ));
        }
        if pr.merge_state == MergeState::Dirty {
            return Err(conflicts(pr.number));
        }
        Ok(UpdateTarget {
            number: pr.number,
            repo: pr.repo.clone(),
            title: pr.title.clone(),
            head_oid: pr.head_oid.clone(),
            method: self.config.prs.update_method,
        })
    }

    pub fn receive_branch_update(
        &mut self,
        number: u64,
        result: &Result<BranchUpdate, String>,
    ) -> bool {
        self.updating = None;
        self.notice = Some(match result {
            Ok(BranchUpdate::Updated) => format!("branch of #{number} updated"),
            Ok(BranchUpdate::UpToDate) => format!("#{number} is already up to date"),
            Ok(BranchUpdate::Conflicts) => conflicts(number),
            Err(message) => format!("update failed for #{number}: {message}"),
        });
        matches!(result, Ok(BranchUpdate::Updated))
    }
}

fn conflicts(number: u64) -> String {
    format!("#{number} has conflicts with its base; resolve them locally")
}
