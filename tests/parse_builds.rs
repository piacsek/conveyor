use std::time::{Duration, UNIX_EPOCH};

use conveyor::model::builds::{BuildStatus, parse_pull_numbers, parse_runs, pr_number_from_title};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/runs.json")).unwrap()
}

#[test]
fn parses_the_verbatim_runs_response_newest_first() {
    let builds = parse_runs(&fixture());

    assert_eq!(builds.len(), 4);
    let latest = &builds[0];
    assert_eq!(latest.id, 34_509_123_916);
    assert_eq!(latest.run_number, 24);
    assert_eq!(latest.status, BuildStatus::Failure);
    assert_eq!(
        latest.sha,
        "b0b5136548f2d7c37a7c09c03e39e34aadad16bd"[..8].to_string() + &latest.sha[8..]
    );
    assert!(latest.sha.starts_with("b0b51365"));
    assert!(
        latest
            .title
            .starts_with("docs: merge queue column in README")
    );
    assert_eq!(latest.actor, "piacsek");
    assert_eq!(
        latest.url,
        "https://github.com/piacsek/conveyor/actions/runs/34509123916"
    );
    assert_eq!(
        latest.started_at,
        Some(UNIX_EPOCH + Duration::from_secs(1_789_061_719))
    );
    assert_eq!(
        latest.finished_at,
        Some(UNIX_EPOCH + Duration::from_secs(1_789_061_947))
    );
    assert_eq!(latest.duration(), Some(Duration::from_secs(228)));
    assert_eq!(latest.pr_number, None, "rebase merges carry no (#N) suffix");
    assert_eq!(builds[1].status, BuildStatus::Success);
}

#[test]
fn statuses_map_from_status_and_conclusion() {
    let value = serde_json::json!({"workflow_runs": [
        {"id": 1, "run_number": 1, "status": "in_progress", "conclusion": null, "head_sha": "a", "display_title": "x", "html_url": "u", "actor": {"login": "me"}, "run_started_at": "2026-09-10T17:35:19Z", "updated_at": "2026-09-10T17:36:19Z"},
        {"id": 2, "run_number": 2, "status": "queued", "conclusion": null, "head_sha": "b", "display_title": "y", "html_url": "u"},
        {"id": 3, "run_number": 3, "status": "completed", "conclusion": "cancelled", "head_sha": "c", "display_title": "z (#77)", "html_url": "u"},
        {"id": 4, "run_number": 4, "status": "completed", "conclusion": "something_new", "head_sha": "d", "display_title": "w", "html_url": "u"}
    ]});
    let builds = parse_runs(&value);
    let statuses: Vec<_> = builds.iter().map(|b| b.status).collect();
    assert_eq!(
        statuses,
        vec![
            BuildStatus::Running,
            BuildStatus::Queued,
            BuildStatus::Cancelled,
            BuildStatus::Unknown
        ]
    );
    assert_eq!(
        builds[0].duration(),
        None,
        "a running build has no duration yet"
    );
    assert_eq!(
        builds[0].elapsed(UNIX_EPOCH + Duration::from_secs(1_789_061_719 + 90)),
        Some(Duration::from_secs(90))
    );
    assert_eq!(builds[2].pr_number, Some(77));
    assert_eq!(builds[2].actor, "");
}

#[test]
fn the_squash_suffix_gives_the_pull_request_number() {
    assert_eq!(pr_number_from_title("feat: thing (#4821)"), Some(4821));
    assert_eq!(pr_number_from_title("feat: thing (#4821) trailing"), None);
    assert_eq!(pr_number_from_title("no number"), None);
}

#[test]
fn the_commit_pulls_response_gives_the_first_pull_request() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/commit-pulls.json")).unwrap();
    let pr = parse_pull_numbers(&value).unwrap();
    assert_eq!(pr.number, 3);
    assert_eq!(pr.title, "Phase 2: merge queue column");
    assert_eq!(pr.author, "piacsek");
    assert_eq!(pr.url, "https://github.com/piacsek/conveyor/pull/3");
    assert!(parse_pull_numbers(&serde_json::json!([])).is_none());
}
