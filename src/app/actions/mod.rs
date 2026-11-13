use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::config::UpdateMethod;
use crate::model::prs::StatusChange;

use super::{Action, App, Mode, StageView};

mod checkout;
mod failures;
mod merge;
mod rerun;
mod status;
mod update;
mod writes;

pub use checkout::CheckoutTarget;
pub use failures::Failure;
pub use merge::MergeTarget;
pub use rerun::Rerun;
pub(super) use rerun::rerun_offer;
pub use status::StatusTarget;
pub use update::UpdateTarget;
pub(super) use update::conflicts;
pub use writes::{Write, WriteKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Merge(MergeTarget),
    Update(UpdateTarget),
    Rerun(Rerun),
    Status(StatusTarget),
    Checkout(CheckoutTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub key: char,
    pub label: String,
    pub choice: Result<Choice, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub title: String,
    pub offers: Vec<Offer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    Merge(MergeTarget),
    UpdateBranch(UpdateTarget),
    Status(StatusTarget),
}

impl App {
    pub(super) fn confirm(&mut self, key: KeyEvent) -> Action {
        let Mode::Confirm(pending) = std::mem::take(&mut self.mode) else {
            return Action::Continue;
        };
        match pending {
            Pending::Merge(target) => self.confirm_merge(key, target),
            Pending::UpdateBranch(target) => self.confirm_update(key, target),
            Pending::Status(target) => self.confirm_status(key, target),
        }
    }

    pub(super) fn ask_to_manage(&mut self) {
        if let Some(busy) = self.write_in_flight() {
            self.notice = Some(busy);
            return;
        }
        match self.menu() {
            Ok(menu) => self.mode = Mode::Manage(menu),
            Err(message) => self.notice = Some(message),
        }
    }

    fn write_in_flight(&self) -> Option<String> {
        self.selected_key()
            .and_then(|key| self.write_on(self.focus, key))
            .map(Write::progress)
    }

    fn menu(&self) -> Result<Menu, String> {
        self.stage(self.focus).menu(&self.context())
    }

    pub(super) fn manage(&mut self, key: KeyEvent) -> Action {
        let Mode::Manage(menu) = std::mem::take(&mut self.mode) else {
            return Action::Continue;
        };
        let picked = match key.code {
            KeyCode::Char(letter) if key.modifiers.is_empty() => menu
                .offers
                .iter()
                .find(|offer| offer.key == letter)
                .cloned(),
            _ => None,
        };
        let Some(picked) = picked else {
            self.notice = Some("nothing changed".to_string());
            return Action::Continue;
        };
        let Ok(choice) = picked.choice.clone() else {
            self.mode = Mode::Manage(menu);
            return Action::Continue;
        };
        match self.menu() {
            Err(message) => {
                self.notice = Some(message);
                return Action::Continue;
            }
            Ok(fresh) if fresh.title != menu.title => {
                self.notice = Some(self.stage(self.focus).changed());
                return Action::Continue;
            }
            Ok(fresh) => match fresh
                .offers
                .into_iter()
                .find(|offer| offer.key == picked.key)
            {
                Some(offer) if offer == picked => {}
                Some(Offer {
                    choice: Err(reason),
                    ..
                }) => {
                    self.notice = Some(reason);
                    return Action::Continue;
                }
                _ => {
                    self.notice = Some(self.stage(self.focus).changed());
                    return Action::Continue;
                }
            },
        }
        match choice {
            Choice::Merge(target) => self.mode = Mode::Confirm(Pending::Merge(target)),
            Choice::Update(target) if target.method == UpdateMethod::Rebase => {
                self.mode = Mode::Confirm(Pending::UpdateBranch(target));
            }
            Choice::Update(target) => return self.send_update(target),
            Choice::Rerun(rerun) => return self.send_rerun(rerun),
            Choice::Status(target)
                if target.change == StatusChange::Close || target.change.lands_a_merge() =>
            {
                self.mode = Mode::Confirm(Pending::Status(target));
            }
            Choice::Status(target) => return self.send_status(target),
            Choice::Checkout(target) => return self.send_checkout(target),
        }
        Action::Continue
    }

    fn reconfirm(&self, key: char, number: u64, expected: Choice) -> Result<(), String> {
        let menu = self.menu()?;
        if menu.title != format!("Manage #{number}") {
            return Err(self.prs.changed());
        }
        match menu.offers.into_iter().find(|offer| offer.key == key) {
            Some(Offer {
                choice: Ok(choice), ..
            }) if choice == expected => Ok(()),
            Some(Offer {
                choice: Err(reason),
                ..
            }) => Err(reason),
            _ => Err(self.prs.changed()),
        }
    }
}

pub(super) fn offer(key: char, label: &str, choice: Result<Choice, String>) -> Offer {
    Offer {
        key,
        label: label.to_string(),
        choice,
    }
}
