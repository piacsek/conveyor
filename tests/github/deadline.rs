use std::fs;
use std::time::Duration;

use conveyor::app::{Input, Rows, Stage};
use conveyor::config::{Config, MergeMethod, UpdateMethod};
use conveyor::log::Log;
use conveyor::model::prs::StatusChange;
use conveyor::sources::failure::{Failure, FailureKind};
use conveyor::sources::fetch::fetch_prs;
use conveyor::sources::github::{CliGh, Github};

use crate::shims::{serialized, shim};
use crate::support::{Harness, fetching};

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

    let failure = time_out_once_ready(&dir, gh, |gh| gh.rest("repos/acme/api").map(|_| ()));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains(" gh api repos/acme/api "), "{text}");
    assert!(
        text.ends_with("ms timed out: dial tcp: i/o timeout\n"),
        "{text}"
    );
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
        "gh did not answer within 2s; it may already be merged, press r"
    );
}

#[test]
fn an_update_past_its_deadline_says_the_branch_may_already_be_updated() {
    let failure = past_the_write_deadline(|gh| {
        gh.update_branch("acme/api", 12, UpdateMethod::Merge)
            .map(|_| ())
    });

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; the branch may already be updated, press r"
    );
}

#[test]
fn marking_ready_past_its_deadline_says_it_may_already_be_marked_ready() {
    let change = StatusChange::Ready;

    let failure = past_the_write_deadline(|gh| gh.change_status("acme/api", 12, &change));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; it may already be marked ready, press r"
    );
}

#[test]
fn converting_to_draft_past_its_deadline_says_it_may_already_be_a_draft() {
    let change = StatusChange::Draft;

    let failure = past_the_write_deadline(|gh| gh.change_status("acme/api", 12, &change));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; it may already be a draft, press r"
    );
}

#[test]
fn enabling_auto_merge_past_its_deadline_says_auto_merge_may_already_be_on() {
    let change = StatusChange::EnableAutoMerge {
        node_id: "PR_kwacme12".into(),
        method: MergeMethod::Squash,
        head_oid: "8c1f2a7d".into(),
    };

    let failure = past_the_write_deadline(|gh| gh.change_status("acme/api", 12, &change));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; auto-merge may already be on, press r"
    );
}

#[test]
fn disabling_auto_merge_past_its_deadline_says_auto_merge_may_already_be_off() {
    let change = StatusChange::DisableAutoMerge {
        node_id: "PR_kwacme12".into(),
    };

    let failure = past_the_write_deadline(|gh| gh.change_status("acme/api", 12, &change));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; auto-merge may already be off, press r"
    );
}

#[test]
fn adding_to_the_merge_queue_past_its_deadline_says_it_may_already_be_in_the_merge_queue() {
    let change = StatusChange::Enqueue {
        node_id: "PR_kwacme12".into(),
        head_oid: "8c1f2a7d".into(),
    };

    let failure = past_the_write_deadline(|gh| gh.change_status("acme/api", 12, &change));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; it may already be in the merge queue, press r"
    );
}

#[test]
fn leaving_the_merge_queue_past_its_deadline_says_it_may_already_be_out_of_the_merge_queue() {
    let change = StatusChange::Dequeue {
        node_id: "PR_kwacme12".into(),
    };

    let failure = past_the_write_deadline(|gh| gh.change_status("acme/api", 12, &change));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; it may already be out of the merge queue, press r"
    );
}

#[test]
fn closing_past_its_deadline_says_it_may_already_be_closed() {
    let change = StatusChange::Close;

    let failure = past_the_write_deadline(|gh| gh.change_status("acme/api", 12, &change));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; it may already be closed, press r"
    );
}

#[test]
fn a_rerun_past_its_deadline_says_the_run_may_already_be_rerunning() {
    let failure = past_the_write_deadline(|gh| gh.rerun_failed("acme/api", 1026));

    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        failure.to_string(),
        "gh did not answer within 2s; the run may already be rerunning, press r"
    );
}

const SHORT: Duration = Duration::from_secs(1);
const LONG: Duration = Duration::from_secs(2);

fn time_out_once_ready(
    dir: &tempfile::TempDir,
    mut gh: CliGh,
    call: impl Fn(&CliGh) -> Result<(), Failure>,
) -> Failure {
    let ready = dir.path().join("bin/gh.ready");
    let mut deadline = Duration::from_millis(300);
    loop {
        gh = gh.with_deadlines(deadline, deadline);
        let failure = call(&gh).unwrap_err();
        if ready.exists() || deadline > Duration::from_secs(30) {
            return failure;
        }
        deadline *= 2;
    }
}

fn past_the_write_deadline(call: impl Fn(&CliGh) -> Result<(), Failure>) -> Failure {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exec sleep 5").with_deadlines(SHORT, LONG);
    call(&gh).unwrap_err()
}
