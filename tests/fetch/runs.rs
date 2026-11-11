use std::io;

use crate::fakes::FakeGithub;

#[test]
fn fetch_jobs_reads_the_runs_jobs_route_and_parses_them() {
    use conveyor::sources::fetch::fetch_jobs;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![(
        "repos/acme/webapp/actions/runs/1025/jobs".to_string(),
        serde_json::from_str(include_str!("../fixtures/jobs.json")).unwrap(),
    )];

    let jobs = fetch_jobs(&gh, "acme/webapp", 1025).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].name, "check");
    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls.first().map(|(q, _)| q.as_str()),
        Some("GET repos/acme/webapp/actions/runs/1025/jobs?per_page=100")
    );
}

#[test]
fn fetch_jobs_turns_a_gh_error_into_a_message() {
    let gh = FakeGithub {
        rest_response: Err(io::Error::other("gh: HTTP 404")),
        ..FakeGithub::with_response(Ok(serde_json::Value::Null))
    };

    let error = conveyor::sources::fetch::fetch_jobs(&gh, "acme/webapp", 7).unwrap_err();

    assert_eq!(error, "gh: HTTP 404");
}

#[test]
fn fetch_log_tails_the_failed_steps_output_for_the_job() {
    use conveyor::sources::fetch::fetch_log;
    let gh = FakeGithub {
        log_response: Ok("check\tRun cargo test\tone\ncheck\tRun cargo test\ttwo\n".to_string()),
        ..FakeGithub::with_response(Ok(serde_json::Value::Null))
    };

    let lines = fetch_log(&gh, "acme/webapp", 1026, 7).unwrap();

    assert_eq!(lines, vec!["one", "two"]);
    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls.first().map(|(q, _)| q.as_str()),
        Some("LOG acme/webapp 1026 7")
    );
}

#[test]
fn fetch_log_turns_a_gh_error_into_a_message() {
    let gh = FakeGithub {
        log_response: Err(io::Error::other("gh: HTTP 404")),
        ..FakeGithub::with_response(Ok(serde_json::Value::Null))
    };

    let error = conveyor::sources::fetch::fetch_log(&gh, "acme/webapp", 7, 1).unwrap_err();

    assert_eq!(error, "gh: HTTP 404");
}

#[test]
fn rerun_failed_asks_once_per_run_in_order() {
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));

    let results = conveyor::sources::fetch::rerun_failed(&gh, "acme/webapp", &[431, 432]);

    assert_eq!(results, vec![(431, Ok(())), (432, Ok(()))]);
    let calls = gh.calls.borrow();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "RERUN acme/webapp 431");
    assert_eq!(calls[1].0, "RERUN acme/webapp 432");
}

#[test]
fn rerun_failed_tries_every_run_past_an_error_and_reports_each() {
    let gh = FakeGithub {
        rerun_failures: vec![(432, "HTTP 403: Resource not accessible".to_string())],
        ..FakeGithub::with_response(Ok(serde_json::Value::Null))
    };

    let results = conveyor::sources::fetch::rerun_failed(&gh, "acme/webapp", &[431, 432, 433]);

    assert_eq!(gh.calls.borrow().len(), 3);
    assert_eq!(results[0], (431, Ok(())));
    assert_eq!(
        results[1],
        (432, Err("HTTP 403: Resource not accessible".to_string()))
    );
    assert_eq!(results[2], (433, Ok(())));
}
