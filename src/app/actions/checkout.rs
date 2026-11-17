use std::path::PathBuf;

use crate::app::{Action, App, pull_key};

use super::WriteKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutTarget {
    pub number: u64,
    pub repo: String,
    pub dir: Option<PathBuf>,
}

impl CheckoutTarget {
    pub fn key(&self) -> u64 {
        pull_key(&self.repo, self.number)
    }
}

impl App {
    pub fn receive_checkout(
        &mut self,
        target: &CheckoutTarget,
        result: &Result<(), crate::sources::failure::Failure>,
    ) {
        let number = target.number;
        let write = self.finish_write(target.key(), &WriteKind::Checkout(number));
        if let Err(message) = result {
            self.fail(write, message);
        }
        self.notice = Some(match (result, &target.dir) {
            (Ok(()), Some(dir)) => format!("#{number} checked out in {}", dir.display()),
            (Ok(()), None) => format!("#{number} checked out in the current directory"),
            (Err(message), _) => format!("#{number}: {message}"),
        });
    }

    pub(super) fn send_checkout(&mut self, target: CheckoutTarget) -> Action {
        self.start_write(target.key(), WriteKind::Checkout(target.number));
        Action::Checkout(target)
    }
}
