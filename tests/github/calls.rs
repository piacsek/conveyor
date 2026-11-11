use std::fs;

use conveyor::log::Log;
use conveyor::sources::github::{CliGh, Github};

use crate::shims::{serialized, shim};

#[test]
fn a_call_made_while_another_hangs_is_not_held_up_by_it() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = std::sync::Arc::new(shim(
        &dir,
        r#"case "$*" in *hung*) sleep 2;; *) echo '{}';; esac"#,
    ));
    let mut held_up = 0;

    for _ in 0..150 {
        let start = std::sync::Arc::new(std::sync::Barrier::new(4));
        std::thread::spawn({
            let (gh, start) = (gh.clone(), start.clone());
            move || {
                start.wait();
                let _ = gh.rest("hung");
            }
        });
        let healthy: Vec<_> = (0..3)
            .map(|_| {
                let (gh, start) = (gh.clone(), start.clone());
                std::thread::spawn(move || {
                    start.wait();
                    let started = std::time::Instant::now();
                    let answer = gh.rest("repos/acme/webapp");
                    answer.is_err() || started.elapsed() > std::time::Duration::from_secs(1)
                })
            })
            .collect();
        held_up += healthy
            .into_iter()
            .map(|call| call.join().unwrap())
            .filter(|held_up| *held_up)
            .count();
    }

    assert_eq!(held_up, 0);
}

#[test]
fn a_failing_call_is_logged_with_its_argv_duration_and_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let gh = shim(
        &dir,
        "echo 'gh: HTTP 502' >&2; echo 'retry later' >&2; exit 1",
    )
    .with_log(Log::open(&file));

    let _ = gh.graphql("query { x }", &[("q", "is:pr".into())]);

    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains(" gh api graphql -f query=<11 chars> -f q=is:pr "),
        "{text}"
    );
    assert!(
        text.ends_with("ms exit 1: gh: HTTP 502 | retry later\n"),
        "{text}"
    );
}

#[test]
fn a_successful_call_is_logged_as_ok_without_its_stdout() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let gh = shim(&dir, r#"echo '{"secret":"payload"}'"#).with_log(Log::open(&file));

    let _ = gh.rest("repos/acme/webapp/actions/runs");

    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains(" gh api repos/acme/webapp/actions/runs "),
        "{text}"
    );
    assert!(text.ends_with("ms ok\n"), "{text}");
    assert!(!text.contains("payload"), "{text}");
}

#[test]
fn graphql_argv_passes_the_query_file_and_typed_variables() {
    let args = CliGh::graphql_args(
        "query($q: String!, $first: Int!) { x }",
        &[("q", "is:pr".into()), ("first", 20u64.into())],
    );
    assert_eq!(
        args,
        vec![
            "api",
            "graphql",
            "-f",
            "query=query($q: String!, $first: Int!) { x }",
            "-f",
            "q=is:pr",
            "-F",
            "first=20",
        ]
    );
}

#[test]
fn graphql_returns_the_parsed_json_from_stdout() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, r#"echo '{"data":{"ok":true}}'"#);

    let value = gh.graphql("query { x }", &[]).unwrap();

    assert_eq!(
        value.pointer("/data/ok"),
        Some(&serde_json::Value::Bool(true))
    );
}

#[test]
fn a_failing_gh_reports_its_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo 'gh: HTTP 401: Bad credentials' >&2; exit 1");

    let err = gh.graphql("query { x }", &[]).unwrap_err();

    assert_eq!(err.to_string(), "gh: HTTP 401: Bad credentials");
}

#[test]
fn a_missing_gh_binary_says_how_to_install_it() {
    let gh = CliGh::with_program("/nonexistent/gh".into());

    let err = gh.graphql("query { x }", &[]).unwrap_err();

    assert_eq!(
        err.to_string(),
        "gh not found on PATH; install gh (brew install gh) and run gh auth login"
    );
}

#[test]
fn current_repo_asks_gh_for_the_checked_out_repository() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::current_repo_args(),
        vec![
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "--jq",
            ".nameWithOwner"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo acme/webapp");
    assert_eq!(gh.current_repo().unwrap(), "acme/webapp");
}

#[test]
fn rest_argv_and_json_parsing() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::rest_args("repos/acme/webapp/actions/runs?event=merge_group&per_page=20"),
        vec![
            "api",
            "repos/acme/webapp/actions/runs?event=merge_group&per_page=20"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, r#"echo '{"total_count":0,"workflow_runs":[]}'"#);
    let value = gh.rest("repos/acme/webapp/actions/runs").unwrap();
    assert_eq!(value["total_count"], 0);
}

#[test]
fn log_failed_argv_views_the_job_with_gh_run_view_and_returns_the_raw_text() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::log_failed_args("acme/webapp", 1026, 7),
        vec![
            "run",
            "view",
            "1026",
            "--job",
            "7",
            "--log-failed",
            "-R",
            "acme/webapp"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, r"printf 'check\tRun\tboom\n'");
    assert_eq!(
        gh.log_failed("acme/webapp", 1026, 7).unwrap(),
        "check\tRun\tboom\n"
    );
}
