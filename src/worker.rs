use std::io;
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::app::{CheckoutTarget, Input, MergeTarget, StatusTarget, UpdateTarget};
use crate::crash;
use crate::sources::failure::Failure;

pub trait Task: Clone + Send + 'static {
    const WORKER: &'static str;

    fn crashed(self, failure: Failure) -> Input;
}

impl Task for (String, u64) {
    const WORKER: &'static str = "jobs";

    fn crashed(self, failure: Failure) -> Input {
        Input::Jobs(self.1, Err(failure))
    }
}

impl Task for (String, u64, u64) {
    const WORKER: &'static str = "log";

    fn crashed(self, failure: Failure) -> Input {
        Input::Log(self.2, Err(failure))
    }
}

impl Task for (String, Vec<u64>) {
    const WORKER: &'static str = "rerun";

    fn crashed(self, failure: Failure) -> Input {
        Input::Rerun(
            self.1
                .into_iter()
                .map(|run_id| (run_id, Err(failure.clone())))
                .collect(),
        )
    }
}

impl Task for MergeTarget {
    const WORKER: &'static str = "merge";

    fn crashed(self, failure: Failure) -> Input {
        Input::Merged(self.number, Err(failure))
    }
}

impl Task for UpdateTarget {
    const WORKER: &'static str = "update";

    fn crashed(self, failure: Failure) -> Input {
        Input::BranchUpdated(self.number, Err(failure))
    }
}

impl Task for StatusTarget {
    const WORKER: &'static str = "status";

    fn crashed(self, failure: Failure) -> Input {
        Input::StatusChanged(self.number, self.change, Err(failure))
    }
}

impl Task for CheckoutTarget {
    const WORKER: &'static str = "checkout";

    fn crashed(self, failure: Failure) -> Input {
        Input::CheckedOut(self, Err(failure))
    }
}

pub fn spawn<T, F>(tx: Sender<io::Result<Input>>, mut work: F) -> Sender<T>
where
    T: Task,
    F: FnMut(T) -> Input + Send + 'static,
{
    let (requests_tx, requests) = mpsc::channel::<T>();
    thread::Builder::new()
        .name(format!("{} worker", T::WORKER))
        .spawn(move || {
            while let Ok(request) = requests.recv() {
                let input = crash::catch(|| work(request.clone()))
                    .unwrap_or_else(|failure| request.crashed(failure));
                if tx.send(Ok(input)).is_err() {
                    return;
                }
            }
        })
        .expect("the worker thread starts");
    requests_tx
}
