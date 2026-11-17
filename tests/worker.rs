use std::io;
use std::sync::mpsc;
use std::time::Duration;

use conveyor::app::{CheckoutTarget, Input, MergeTarget, StatusTarget, UpdateTarget};
use conveyor::config::UpdateMethod;
use conveyor::model::prs::{BranchUpdate, PullRequest, StatusChange};
use conveyor::sources::failure::{Failure, FailureKind};
use conveyor::worker;

mod support;

use ratatui::crossterm::event::KeyCode;
use support::{Harness, checkout_target, key, merge_target, pr, prs, status_target, update_target};

#[test]
fn a_request_comes_back_as_the_input_the_work_built() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |target: MergeTarget| {
        Input::Merged(
            target.number,
            Err(format!("asked for {}", target.number).into()),
        )
    });

    requests.send(merge_target(7, "Add a cache")).unwrap();

    let input = inputs.recv_timeout(WAIT).unwrap().unwrap();
    assert!(
        matches!(input, Input::Merged(7, Err(ref reason)) if reason.to_string() == "asked for 7")
    );
}

#[test]
fn two_requests_come_back_as_two_inputs_in_order() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |target: MergeTarget| {
        Input::Merged(target.number, Ok(()))
    });

    requests.send(merge_target(1, "Add a cache")).unwrap();
    requests.send(merge_target(2, "Drop a route")).unwrap();

    let first = inputs.recv_timeout(WAIT).unwrap().unwrap();
    let second = inputs.recv_timeout(WAIT).unwrap().unwrap();
    assert!(matches!(first, Input::Merged(1, Ok(()))));
    assert!(matches!(second, Input::Merged(2, Ok(()))));
}

#[test]
fn the_worker_ends_once_nobody_reads_its_inputs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let (ended_tx, ended) = mpsc::channel();
    let guard = EndGuard(ended_tx);
    let requests = worker::spawn(inputs_tx, move |target: MergeTarget| {
        let _ = &guard;
        Input::Merged(target.number, Ok(()))
    });
    drop(inputs);

    requests.send(merge_target(7, "Add a cache")).unwrap();

    assert!(ended.recv_timeout(WAIT).is_ok());
}

#[test]
fn the_worker_ends_once_nobody_sends_it_requests() {
    let (inputs_tx, _inputs) = mpsc::channel();
    let (ended_tx, ended) = mpsc::channel();
    let guard = EndGuard(ended_tx);
    let requests = worker::spawn(inputs_tx, move |target: MergeTarget| {
        let _ = &guard;
        Input::Merged(target.number, Ok(()))
    });

    drop(requests);

    assert!(ended.recv_timeout(WAIT).is_ok());
}

#[test]
fn a_crashed_jobs_fetch_fails_that_run_and_the_next_run_gets_its_jobs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |(_, run_id): (String, u64)| {
        if run_id == 1 {
            panic!("boom");
        }
        Input::Jobs(run_id, Ok(Vec::new()))
    });

    requests.send(("acme/webapp".to_string(), 1)).unwrap();
    requests.send(("acme/webapp".to_string(), 2)).unwrap();

    let Input::Jobs(run_id, Err(failure)) = next(&inputs) else {
        panic!("the crash did not come back as a failed jobs fetch");
    };
    assert_eq!(run_id, 1);
    assert_eq!(failure.to_string(), "crashed: boom");
    assert_eq!(failure.kind(), FailureKind::Crashed);
    assert_eq!(next(&inputs), Input::Jobs(2, Ok(Vec::new())));
}

#[test]
fn a_crashed_log_fetch_fails_that_job_and_the_next_log_loads() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |(_, _, job_id): (String, u64, u64)| {
        if job_id == 10 {
            panic!("boom");
        }
        Input::Log(job_id, Ok(vec!["ok".to_string()]))
    });

    requests.send(("acme/webapp".to_string(), 1, 10)).unwrap();
    requests.send(("acme/webapp".to_string(), 1, 11)).unwrap();

    assert_eq!(next(&inputs), Input::Log(10, Err(crashed("boom"))));
    assert_eq!(next(&inputs), Input::Log(11, Ok(vec!["ok".to_string()])));
}

#[test]
fn a_crashed_rerun_fails_every_run_it_asked_for_and_the_next_rerun_runs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |(_, run_ids): (String, Vec<u64>)| {
        if run_ids.contains(&1) {
            panic!("boom");
        }
        Input::Rerun(run_ids.into_iter().map(|run_id| (run_id, Ok(()))).collect())
    });

    requests
        .send(("acme/webapp".to_string(), vec![1, 2]))
        .unwrap();
    requests.send(("acme/webapp".to_string(), vec![3])).unwrap();

    assert_eq!(
        next(&inputs),
        Input::Rerun(vec![(1, Err(crashed("boom"))), (2, Err(crashed("boom")))])
    );
    assert_eq!(next(&inputs), Input::Rerun(vec![(3, Ok(()))]));
}

#[test]
fn a_crashed_merge_fails_that_pull_request_and_the_next_merge_runs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |target: MergeTarget| {
        if target.number == 1 {
            panic!("boom");
        }
        Input::Merged(target.number, Ok(()))
    });

    requests.send(merge_target(1, "Add a cache")).unwrap();
    requests.send(merge_target(2, "Drop a route")).unwrap();

    assert_eq!(next(&inputs), Input::Merged(1, Err(crashed("boom"))));
    assert_eq!(next(&inputs), Input::Merged(2, Ok(())));
}

#[test]
fn a_crashed_update_fails_that_pull_request_and_the_next_update_runs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |target: UpdateTarget| {
        if target.number == 1 {
            panic!("boom");
        }
        Input::BranchUpdated(target.number, Ok(BranchUpdate::Updated))
    });

    requests
        .send(update_target(1, "Add a cache", UpdateMethod::Merge))
        .unwrap();
    requests
        .send(update_target(2, "Drop a route", UpdateMethod::Merge))
        .unwrap();

    assert_eq!(next(&inputs), Input::BranchUpdated(1, Err(crashed("boom"))));
    assert_eq!(
        next(&inputs),
        Input::BranchUpdated(2, Ok(BranchUpdate::Updated))
    );
}

#[test]
fn a_crashed_status_change_fails_that_pull_request_and_the_next_change_runs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |target: StatusTarget| {
        if target.number == 1 {
            panic!("boom");
        }
        Input::StatusChanged(target.number, target.change, Ok(()))
    });

    requests
        .send(status_target(1, "Add a cache", StatusChange::Ready))
        .unwrap();
    requests
        .send(status_target(2, "Drop a route", StatusChange::Draft))
        .unwrap();

    assert_eq!(
        next(&inputs),
        Input::StatusChanged(1, StatusChange::Ready, Err(crashed("boom")))
    );
    assert_eq!(
        next(&inputs),
        Input::StatusChanged(2, StatusChange::Draft, Ok(()))
    );
}

#[test]
fn a_crashed_checkout_fails_that_pull_request_and_the_next_checkout_runs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |target: CheckoutTarget| {
        if target.number == 1 {
            panic!("boom");
        }
        Input::CheckedOut(target, Ok(()))
    });

    requests.send(checkout_target(1, None)).unwrap();
    requests.send(checkout_target(2, None)).unwrap();

    assert_eq!(
        next(&inputs),
        Input::CheckedOut(checkout_target(1, None), Err(crashed("boom")))
    );
    assert_eq!(
        next(&inputs),
        Input::CheckedOut(checkout_target(2, None), Ok(()))
    );
}

#[test]
fn a_panic_that_carries_no_text_still_fails_its_request() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |_: MergeTarget| -> Input {
        std::panic::panic_any(7_u8);
    });

    requests.send(merge_target(1, "Add a cache")).unwrap();

    assert_eq!(
        next(&inputs),
        Input::Merged(1, Err(crashed("a panic with no message")))
    );
}

#[test]
fn a_crashed_merge_marks_its_card_with_the_crash() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |_: MergeTarget| -> Input {
        panic!("boom");
    });
    let mut h = Harness::new();
    h.run(vec![
        prs(vec![mergeable(1, "Add a cache")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    requests.send(merge_target(1, "Add a cache")).unwrap();
    h.run(vec![Ok(next(&inputs))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("! merge failed: crashed: boom"), "{screen}");
}

const WAIT: Duration = Duration::from_secs(5);

fn next(inputs: &mpsc::Receiver<io::Result<Input>>) -> Input {
    inputs.recv_timeout(WAIT).unwrap().unwrap()
}

fn crashed(message: &str) -> Failure {
    Failure::crashed(&message.to_string())
}

fn mergeable(number: u64, title: &str) -> PullRequest {
    let mut pull = pr(number, title);
    pull.head_oid = format!("8c1f2a7d{number}");
    pull
}

struct EndGuard(mpsc::Sender<()>);

impl Drop for EndGuard {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
