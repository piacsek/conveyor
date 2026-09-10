use conveyor::model::builds::BuildStatus;
use conveyor::model::jobs::parse_jobs;

#[test]
fn jobs_parse_failures_first_with_the_failed_step_and_the_duration() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/jobs.json")).unwrap();

    let jobs = parse_jobs(&value);

    assert_eq!(jobs.len(), 2);
    let failed = &jobs[0];
    assert_eq!(failed.name, "check");
    assert_eq!(failed.status, BuildStatus::Failure);
    assert_eq!(failed.failed_step.as_deref(), Some("Run cargo test"));
    assert_eq!(failed.duration().map(|d| d.as_secs()), Some(19));
    assert!(failed.url.ends_with("/job/102978569297"), "{}", failed.url);
    assert_eq!(jobs[1].name, "audit");
    assert_eq!(jobs[1].status, BuildStatus::Success);
    assert_eq!(jobs[1].failed_step, None);
}

#[test]
fn a_running_job_sorts_before_the_settled_ones_and_has_no_duration() {
    let value = serde_json::json!({"jobs": [
        {"id": 1, "name": "done", "status": "completed", "conclusion": "success",
         "started_at": "2026-09-10T17:35:22Z", "completed_at": "2026-09-10T17:35:41Z",
         "html_url": "https://ci/1", "steps": []},
        {"id": 2, "name": "now", "status": "in_progress", "conclusion": null,
         "started_at": "2026-09-10T17:35:22Z", "completed_at": null,
         "html_url": "https://ci/2", "steps": []}
    ]});

    let jobs = parse_jobs(&value);

    assert_eq!(
        jobs.iter().map(|job| job.name.as_str()).collect::<Vec<_>>(),
        vec!["now", "done"]
    );
    assert_eq!(jobs[0].status, BuildStatus::Running);
    assert_eq!(jobs[0].duration(), None);
}

#[test]
fn a_response_without_jobs_is_empty() {
    assert!(parse_jobs(&serde_json::json!({"total_count": 0})).is_empty());
}
