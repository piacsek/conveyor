use std::path::PathBuf;

use crate::app::{Action, App};

use super::WriteKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutTarget {
    pub number: u64,
    pub repo: String,
    pub dir: Option<PathBuf>,
}

impl App {
    pub fn receive_checkout(&mut self, target: &CheckoutTarget, result: &Result<(), String>) {
        let number = target.number;
        let write = self.finish_write(number, &WriteKind::Checkout);
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
        self.start_write(target.number, WriteKind::Checkout);
        Action::Checkout(target)
    }
}
