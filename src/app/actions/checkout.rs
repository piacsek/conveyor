use std::path::PathBuf;

use crate::app::{Action, App};
use crate::model::prs::PullRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutTarget {
    pub number: u64,
    pub repo: String,
    pub dir: Option<PathBuf>,
}

impl App {
    pub(super) fn checkout_target(&self, pr: &PullRequest) -> CheckoutTarget {
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

    pub(super) fn send_checkout(&mut self, target: CheckoutTarget) -> Action {
        self.checking_out = Some(target.number);
        self.notice = Some(format!("checking out #{}…", target.number));
        Action::Checkout(target)
    }
}
