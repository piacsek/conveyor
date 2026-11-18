use std::fs;
use std::time::Duration;

use conveyor::app::{Input, Rows, Stage};
use conveyor::config::{Config, MergeMethod, UpdateMethod};
use conveyor::log::Log;
use conveyor::model::prs::StatusChange;
use conveyor::sources::failure::{Failure, FailureKind};
use conveyor::sources::fetch::fetch_prs;
use conveyor::sources::github::{Aftermath, Github};

use crate::shims::{RECORD_PGID, own_pgid, recorded_pgid, serialized, shim};
use crate::support::{Harness, fetching, time_out_once_ready};

#[test]
fn a_fetch_whose_gh_never_answers_shows_the_timeout_on_its_column_instead_of_a_spinner() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exec sleep 5").with_deadlines(SHORT, LONG);
    let mut h = Harness::new();

    let fetched = fetch_prs(&gh, &Config::default(), 0).map(Rows::Prs);
    h.run(vec![
        fetching(Stage::Prs),
        Ok(Input::Data(Stage::Prs, 0, fetched)),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs ⚠"), "{screen}");
    let body = screen.lines().nth(1).unwrap();
    assert!(!body.starts_with("│fetching…"), "{screen}");
    assert!(body.contains("gh did not answer within 1s"), "{screen}");
}

#[test]
fn a_read_past_its_deadline_is_timed_out_and_logged_with_what_gh_had_said() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let gh = shim(
        &dir,
        "echo 'dial tcp: i/o timeout' >&2\n: > \"$0.ready\"\nexec sleep 5",
    )
    .with_log(Log::open(&file));

    let failure = time_out_once_ready(
        &dir.path().join("bin/gh.ready"),
        gh,
        |gh, deadline| gh.with_deadlines(deadline, deadline),
        |gh| gh.rest("repos/acme/api"),
    )
    .unwrap_err();

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains(" gh api repos/acme/api "), "{text}");
    assert!(
        text.ends_with("ms timed out: dial tcp: i/o timeout\n"),
        "{text}"
    );
}

#[test]
fn a_read_runs_in_a_process_group_of_its_own_that_its_deadline_can_kill() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, &format!("{RECORD_PGID}\necho '{{}}'"));

    let answer = gh.rest("repos/acme/api");

    assert!(answer.is_ok(), "{answer:?}");

    assert_ne!(recorded_pgid(&dir), own_pgid());
}

#[test]
fn the_failed_log_waits_as_long_as_a_write_since_it_downloads_the_whole_log() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "sleep 0.3\necho 'the log'").with_deadlines(BRIEF, LONG);

    let log = gh.log_failed("acme/api", 70, 7);

    assert_eq!(log.unwrap(), "the log\n");
}

#[test]
fn a_merge_past_its_deadline_says_it_may_be_merged() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exec sleep 5").with_deadlines(LONG, SHORT);

    let failure = gh
        .merge("acme/api", 12, MergeMethod::Squash, "8c1f2a7d")
        .unwrap_err();

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(failure.to_string(), "may be merged: no answer in 1s");
}

#[test]
fn every_other_write_past_its_deadline_is_timed_out() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exec sleep 5").with_deadlines(LONG, BRIEF);

    let (update, rerun, statuses) = std::thread::scope(|scope| {
        let update = scope.spawn(|| gh.update_branch("acme/api", 12, UpdateMethod::Merge));
        let rerun = scope.spawn(|| gh.rerun_failed("acme/api", 4821));
        let statuses = [
            StatusChange::Ready,
            StatusChange::Draft,
            auto_merge(true),
            auto_merge(false),
            queue(true),
            queue(false),
            StatusChange::Close,
        ]
        .map(|change| {
            let gh = &gh;
            scope.spawn(move || gh.change_status("acme/api", 12, &change))
        });
        (
            update.join().unwrap(),
            rerun.join().unwrap(),
            statuses.map(|status| status.join().unwrap()),
        )
    });
    let [
        ready,
        draft,
        auto_merge_on,
        auto_merge_off,
        enqueue,
        dequeue,
        close,
    ] = statuses;

    assert_eq!(update.as_ref().unwrap_err().kind(), FailureKind::TimedOut);
    assert_eq!(rerun.as_ref().unwrap_err().kind(), FailureKind::TimedOut);
    assert_eq!(ready.as_ref().unwrap_err().kind(), FailureKind::TimedOut);
    assert_eq!(draft.as_ref().unwrap_err().kind(), FailureKind::TimedOut);
    assert_eq!(
        auto_merge_on.as_ref().unwrap_err().kind(),
        FailureKind::TimedOut
    );
    assert_eq!(
        auto_merge_off.as_ref().unwrap_err().kind(),
        FailureKind::TimedOut
    );
    assert_eq!(enqueue.as_ref().unwrap_err().kind(), FailureKind::TimedOut);
    assert_eq!(dequeue.as_ref().unwrap_err().kind(), FailureKind::TimedOut);
    assert_eq!(close.as_ref().unwrap_err().kind(), FailureKind::TimedOut);
    assert_eq!(aftermath(&update), Some("branch may be updated"));
    assert_eq!(aftermath(&rerun), Some("may be rerunning"));
    assert_eq!(aftermath(&ready), Some("may be marked ready"));
    assert_eq!(aftermath(&draft), Some("may be a draft"));
    assert_eq!(aftermath(&auto_merge_on), Some("auto-merge may be on"));
    assert_eq!(aftermath(&auto_merge_off), Some("auto-merge may be off"));
    assert_eq!(aftermath(&enqueue), Some("may be in the queue"));
    assert_eq!(aftermath(&dequeue), Some("may be out of the queue"));
    assert_eq!(aftermath(&close), Some("may be closed"));
}

#[test]
fn a_timed_out_update_says_the_branch_may_be_updated() {
    assert_eq!(Aftermath::Update.wording(), "branch may be updated");
}

#[test]
fn a_timed_out_rerun_says_it_may_be_rerunning() {
    assert_eq!(Aftermath::Rerun.wording(), "may be rerunning");
}

#[test]
fn a_timed_out_status_change_says_what_may_have_happened() {
    assert_eq!(
        Aftermath::Status(&StatusChange::Ready).wording(),
        "may be marked ready"
    );
    assert_eq!(
        Aftermath::Status(&StatusChange::Draft).wording(),
        "may be a draft"
    );
    assert_eq!(
        Aftermath::Status(&auto_merge(true)).wording(),
        "auto-merge may be on"
    );
    assert_eq!(
        Aftermath::Status(&auto_merge(false)).wording(),
        "auto-merge may be off"
    );
    assert_eq!(
        Aftermath::Status(&queue(true)).wording(),
        "may be in the queue"
    );
    assert_eq!(
        Aftermath::Status(&queue(false)).wording(),
        "may be out of the queue"
    );
    assert_eq!(
        Aftermath::Status(&StatusChange::Close).wording(),
        "may be closed"
    );
    assert_eq!(
        Aftermath::Status(&StatusChange::Approve {
            node_id: "PR_kwacme12".into(),
            head_oid: "8c1f2a7d".into(),
        })
        .wording(),
        "may be approved"
    );
}

const SHORT: Duration = Duration::from_secs(1);
const LONG: Duration = Duration::from_secs(2);
const BRIEF: Duration = Duration::from_millis(100);

fn auto_merge(on: bool) -> StatusChange {
    if on {
        StatusChange::EnableAutoMerge {
            node_id: "PR_kwacme12".into(),
            method: MergeMethod::Squash,
            head_oid: "8c1f2a7d".into(),
        }
    } else {
        StatusChange::DisableAutoMerge {
            node_id: "PR_kwacme12".into(),
        }
    }
}

fn queue(join: bool) -> StatusChange {
    if join {
        StatusChange::Enqueue {
            node_id: "PR_kwacme12".into(),
            head_oid: "8c1f2a7d".into(),
        }
    } else {
        StatusChange::Dequeue {
            node_id: "PR_kwacme12".into(),
        }
    }
}

fn aftermath<T>(result: &Result<T, Failure>) -> Option<&'static str> {
    result
        .as_ref()
        .err()?
        .unanswered_write()
        .map(|unanswered| unanswered.aftermath)
}
