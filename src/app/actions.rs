use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::config::UpdateMethod;
use crate::model::builds::BuildStatus;
use crate::model::prs::{BranchUpdate, MergeState, PullRequest, StatusChange};

use super::{Action, App, Mode, Stage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutTarget {
    pub number: u64,
    pub repo: String,
    pub dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub change: StatusChange,
}

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
pub struct UpdateTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
    pub method: UpdateMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTarget {
    pub number: u64,
    pub repo: String,
    pub title: String,
    pub head_oid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rerun {
    pub repo: String,
    pub run_ids: Vec<u64>,
    pub label: String,
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
        if let Some(number) = self.merging {
            return Some(format!("merging #{number}…"));
        }
        if let Some(number) = self.updating {
            return Some(format!("updating #{number}…"));
        }
        if let Some((number, change)) = &self.changing {
            return Some(progress(*number, change));
        }
        if let Some(number) = self.checking_out {
            return Some(format!("checking out #{number}…"));
        }
        self.rerunning
            .as_ref()
            .map(|(_, label)| format!("rerunning {label}…"))
    }

    fn menu(&self) -> Result<Menu, String> {
        if matches!(self.focus, Stage::Prs | Stage::Builds) && self.focused_error().is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        match self.focus {
            Stage::Prs => {
                let pr = self.prs.selected().ok_or("nothing selected")?;
                Ok(Menu {
                    title: format!("Manage #{}", pr.number),
                    offers: self.pr_offers(pr),
                })
            }
            Stage::Builds => {
                let build = self.builds.selected().ok_or("nothing selected")?;
                Ok(Menu {
                    title: format!("Manage run {}", build.run_number),
                    offers: vec![self.rerun_offer()],
                })
            }
            Stage::Queue | Stage::Deployed => Err("S manages My PRs and Main builds".to_string()),
        }
    }

    fn rerun_offer(&self) -> Offer {
        let target = self.rerun_target();
        let label = match &target {
            Ok(rerun) if rerun.run_ids.len() > 1 => {
                format!("rerun failed jobs ({} runs)", rerun.run_ids.len())
            }
            _ => "rerun failed jobs".to_string(),
        };
        offer('f', &label, target.map(Choice::Rerun))
    }

    fn pr_offers(&self, pr: &PullRequest) -> Vec<Offer> {
        let status = |change| {
            Ok(Choice::Status(StatusTarget {
                number: pr.number,
                repo: pr.repo.clone(),
                title: pr.title.clone(),
                head_oid: pr.head_oid.clone(),
                change,
            }))
        };
        let update = match self.config.prs.update_method {
            UpdateMethod::Merge => "update branch",
            UpdateMethod::Rebase => "rebase branch",
        };
        let queued = pr.queue_position.is_some();
        let draft = if pr.is_draft {
            offer('t', "mark ready", status(StatusChange::Ready))
        } else if queued {
            offer(
                't',
                "convert to draft",
                Err(format!("#{} is in the merge queue", pr.number)),
            )
        } else if let Some((number, _)) = self.merge_landing {
            offer(
                't',
                "convert to draft",
                Err(format!("waiting for My PRs to catch up with #{number}")),
            )
        } else {
            offer('t', "convert to draft", status(StatusChange::Draft))
        };
        let auto = if pr.can_disable_auto_merge {
            offer(
                'a',
                "disable auto-merge",
                status(StatusChange::DisableAutoMerge {
                    node_id: pr.node_id.clone(),
                }),
            )
        } else if pr.can_enable_auto_merge {
            offer(
                'a',
                "enable auto-merge",
                status(StatusChange::EnableAutoMerge {
                    node_id: pr.node_id.clone(),
                    method: self.config.prs.merge_method,
                    head_oid: pr.head_oid.clone(),
                }),
            )
        } else {
            offer(
                'a',
                "enable auto-merge",
                Err(format!("GitHub does not allow it for #{}", pr.number)),
            )
        };
        let dequeue = if queued {
            status(StatusChange::Dequeue {
                node_id: pr.node_id.clone(),
            })
        } else {
            Err(format!("#{} is not queued", pr.number))
        };
        let merge = if pr.merge_queue_enabled {
            offer(
                'm',
                "add to merge queue",
                self.merge_target().and_then(|_| {
                    status(StatusChange::Enqueue {
                        node_id: pr.node_id.clone(),
                        head_oid: pr.head_oid.clone(),
                    })
                }),
            )
        } else {
            offer('m', "merge", self.merge_target().map(Choice::Merge))
        };
        vec![
            merge,
            offer('u', update, self.update_target().map(Choice::Update)),
            self.rerun_offer(),
            draft,
            auto,
            offer('e', "leave the merge queue", dequeue),
            offer(
                'c',
                "check out locally",
                Ok(Choice::Checkout(self.checkout_target(pr))),
            ),
            offer('x', "close", status(StatusChange::Close)),
        ]
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
                self.notice = Some(changed(self.focus));
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
                    self.notice = Some(changed(self.focus));
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

    fn confirm_status(&mut self, key: KeyEvent, target: StatusTarget) -> Action {
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

    fn reconfirm(&self, key: char, number: u64, expected: Choice) -> Result<(), String> {
        let menu = self.menu()?;
        if menu.title != format!("Manage #{number}") {
            return Err(changed(Stage::Prs));
        }
        match menu.offers.into_iter().find(|offer| offer.key == key) {
            Some(Offer {
                choice: Ok(choice), ..
            }) if choice == expected => Ok(()),
            Some(Offer {
                choice: Err(reason),
                ..
            }) => Err(reason),
            _ => Err(changed(Stage::Prs)),
        }
    }

    fn checkout_target(&self, pr: &PullRequest) -> CheckoutTarget {
        CheckoutTarget {
            number: pr.number,
            repo: pr.repo.clone(),
            dir: self
                .config
                .repo
                .iter()
                .filter(|repo| repo.name.eq_ignore_ascii_case(&pr.repo))
                .find_map(|repo| repo.path.clone()),
        }
    }

    pub fn receive_checkout(&mut self, target: &CheckoutTarget, result: &Result<(), String>) {
        self.checking_out = None;
        let number = target.number;
        self.notice = Some(match (result, &target.dir) {
            (Ok(()), Some(dir)) => format!("#{number} checked out in {}", dir.display()),
            (Ok(()), None) => format!("#{number} checked out in the current directory"),
            (Err(message), _) => format!("#{number}: {message}"),
        });
    }

    fn send_checkout(&mut self, target: CheckoutTarget) -> Action {
        self.checking_out = Some(target.number);
        self.notice = Some(format!("checking out #{}…", target.number));
        Action::Checkout(target)
    }

    fn send_status(&mut self, target: StatusTarget) -> Action {
        self.notice = Some(progress(target.number, &target.change));
        self.changing = Some((target.number, target.change.clone()));
        Action::ChangeStatus(target)
    }

    pub fn receive_status(
        &mut self,
        number: u64,
        change: &StatusChange,
        result: &Result<(), String>,
    ) -> bool {
        self.changing = None;
        self.notice = Some(match result {
            Ok(()) => done(number, change),
            Err(message) => format!("#{number}: {message}"),
        });
        if result.is_ok() && change.lands_a_merge() {
            self.merge_landing = Some((number, false));
        }
        result.is_ok()
    }

    fn send_rerun(&mut self, rerun: Rerun) -> Action {
        self.notice = Some(format!("rerunning {}…", rerun.label));
        self.rerunning = Some((self.focus, rerun.label));
        Action::Rerun {
            repo: rerun.repo,
            run_ids: rerun.run_ids,
        }
    }

    pub fn receive_rerun(&mut self, results: Vec<(u64, Result<(), String>)>) -> Option<Stage> {
        let (stage, label) = self.rerunning.take()?;
        let errors: Vec<String> = results
            .iter()
            .filter_map(|(run_id, result)| {
                result
                    .as_ref()
                    .err()
                    .map(|message| format!("run {run_id}: {message}"))
            })
            .collect();
        let reran = results.len() - errors.len();
        self.notice = Some(match (reran, errors.is_empty()) {
            (_, true) => format!("rerun requested: {label}"),
            (0, false) => format!("rerun failed: {}", errors.join("; ")),
            _ => format!(
                "rerun requested for {reran} of {} runs; {}",
                results.len(),
                errors.join("; ")
            ),
        });
        (reran > 0).then_some(stage)
    }

    fn confirm_merge(&mut self, key: KeyEvent, target: MergeTarget) -> Action {
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

    fn merge_target(&self) -> Result<MergeTarget, String> {
        let pr = self.prs.selected().ok_or("nothing selected")?;
        if pr.is_draft {
            return Err(format!("#{} is a draft", pr.number));
        }
        if pr.queue_position.is_some() {
            return Err(format!("#{} is already queued", pr.number));
        }
        Ok(MergeTarget {
            number: pr.number,
            repo: pr.repo.clone(),
            title: pr.title.clone(),
            head_oid: pr.head_oid.clone(),
        })
    }

    pub fn receive_merged(&mut self, number: u64, result: &Result<(), String>) {
        self.merging = None;
        self.merge_landing = result.is_ok().then_some((number, false));
        self.notice = Some(match result {
            Ok(()) => format!("merge sent for #{number}"),
            Err(message) => format!("#{number}: {message}"),
        });
    }

    fn confirm_update(&mut self, key: KeyEvent, target: UpdateTarget) -> Action {
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

    fn send_update(&mut self, target: UpdateTarget) -> Action {
        self.updating = Some(target.number);
        self.notice = Some(format!("updating #{}…", target.number));
        Action::UpdateBranch(target)
    }

    fn update_target(&self) -> Result<UpdateTarget, String> {
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

    fn rerun_target(&self) -> Result<Rerun, String> {
        if matches!(self.focus, Stage::Builds | Stage::Prs) && self.focused_error().is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        match self.focus {
            Stage::Builds => {
                let build = self.builds.selected().ok_or("nothing selected")?;
                let label = format!("run {}", build.run_number);
                let newest = self.builds.all().iter().map(|b| b.run_number).max();
                if newest != Some(build.run_number) {
                    return Err("only the newest main build can be rerun".to_string());
                }
                if !build.is_settled() {
                    return Err(format!("{label} is still running"));
                }
                if !matches!(build.status, BuildStatus::Failure | BuildStatus::Cancelled) {
                    return Err("nothing failed to rerun".to_string());
                }
                let repo = self.builds_repo.clone().ok_or("no repository known")?;
                Ok(Rerun {
                    repo,
                    run_ids: vec![build.id],
                    label,
                })
            }
            Stage::Prs => {
                let pr = self.prs.selected().ok_or("nothing selected")?;
                let run_ids = pr.rerunnable_runs();
                let label = match run_ids.len() {
                    0 => return Err("no finished failing run to rerun".to_string()),
                    1 => format!("#{}", pr.number),
                    runs => format!("#{} ({runs} runs)", pr.number),
                };
                Ok(Rerun {
                    repo: pr.repo.clone(),
                    run_ids,
                    label,
                })
            }
            Stage::Queue => Err("nothing to rerun in the merge queue".to_string()),
            Stage::Deployed => Err("nothing to rerun here".to_string()),
        }
    }
}

fn offer(key: char, label: &str, choice: Result<Choice, String>) -> Offer {
    Offer {
        key,
        label: label.to_string(),
        choice,
    }
}

fn changed(stage: Stage) -> String {
    match stage {
        Stage::Builds => "the run changed; press S again".to_string(),
        _ => "the pull request changed; press S again".to_string(),
    }
}

fn progress(number: u64, change: &StatusChange) -> String {
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

fn conflicts(number: u64) -> String {
    format!("#{number} has conflicts with its base; resolve them locally")
}
