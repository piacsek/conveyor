use std::fs;
use std::time::Duration;

use conveyor::app::{Input, Rows, Stage};
use conveyor::config::{Config, MergeMethod};
use conveyor::log::Log;
use conveyor::model::prs::StatusChange;
use conveyor::sources::failure::FailureKind;
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
fn a_merge_past_its_deadline_says_it_may_already_be_merged() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exec sleep 5").with_deadlines(SHORT, LONG);

    let failure = gh
        .merge("acme/api", 12, MergeMethod::Squash, "8c1f2a7d")
        .unwrap_err();

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; it may already be merged"
    );
}

#[test]
fn a_timed_out_update_says_the_branch_may_already_be_updated() {
    assert_eq!(
        Aftermath::Update.wording(),
        "the branch may already be updated"
    );
}

#[test]
fn a_timed_out_rerun_says_the_run_may_already_be_rerunning() {
    assert_eq!(
        Aftermath::Rerun.wording(),
        "the run may already be rerunning"
    );
}

#[test]
fn a_timed_out_status_change_says_what_may_already_have_happened() {
    assert_eq!(
        Aftermath::Status(&StatusChange::Ready).wording(),
        "it may already be marked ready"
    );
    assert_eq!(
        Aftermath::Status(&StatusChange::Draft).wording(),
        "it may already be a draft"
    );
    assert_eq!(
        Aftermath::Status(&auto_merge(true)).wording(),
        "auto-merge may already be on"
    );
    assert_eq!(
        Aftermath::Status(&auto_merge(false)).wording(),
        "auto-merge may already be off"
    );
    assert_eq!(
        Aftermath::Status(&queue(true)).wording(),
        "it may already be in the merge queue"
    );
    assert_eq!(
        Aftermath::Status(&queue(false)).wording(),
        "it may already be out of the merge queue"
    );
    assert_eq!(
        Aftermath::Status(&StatusChange::Close).wording(),
        "it may already be closed"
    );
}

const SHORT: Duration = Duration::from_secs(1);
const LONG: Duration = Duration::from_secs(2);

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
