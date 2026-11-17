use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use conveyor::app::{App, Input, Rows, Stage};
use conveyor::config::{Config, Repo};
use conveyor::refresh;
use conveyor::sources::failure::{Failure, FailureKind};
use conveyor::sources::fetch::{BuildsSource, fetch_prs, fetch_queue, repos};
use conveyor::sources::github::{CliGh, MISSING};

use crate::shims::{serialized, shim};

const SIGNED_OUT_SAID: &str = "To get started with GitHub CLI, please run:  gh auth login\n\
     Alternatively, populate the GH_TOKEN environment variable with a GitHub API authentication token.";

const SIGNED_OUT: &str = "echo 'To get started with GitHub CLI, please run:  gh auth login' >&2; \
     echo 'Alternatively, populate the GH_TOKEN environment variable with a GitHub API authentication token.' >&2; \
     exit 4";

#[test]
fn a_signed_out_gh_fails_my_prs_as_not_signed_in_with_its_own_words() {
    let app = my_prs_failed_by(SIGNED_OUT);

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(app.focused_error(), Some(SIGNED_OUT_SAID));
}

#[test]
fn a_rejected_token_fails_my_prs_as_not_signed_in_with_its_own_words() {
    let app = my_prs_failed_by("echo 'gh: Bad credentials (HTTP 401)' >&2; exit 1");

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(app.focused_error(), Some("gh: Bad credentials (HTTP 401)"));
}

#[test]
fn a_rejected_token_on_a_pull_request_view_is_not_signed_in_with_its_own_words() {
    let app = my_prs_failed_by(
        "echo 'HTTP 401: Bad credentials (https://api.github.com/graphql)' >&2; \
         echo 'Try authenticating with:  gh auth login -h github.com' >&2; exit 1",
    );

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(
        app.focused_error(),
        Some(
            "HTTP 401: Bad credentials (https://api.github.com/graphql)\n\
             Try authenticating with:  gh auth login -h github.com"
        )
    );
}

#[test]
fn a_spent_rate_limit_fails_my_prs_as_rate_limited_with_its_own_words() {
    let app = my_prs_failed_by(
        "echo 'gh: API rate limit exceeded for user ID 1. (HTTP 403)' >&2; exit 1",
    );

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::RateLimited);
    assert_eq!(
        app.focused_error(),
        Some("gh: API rate limit exceeded for user ID 1. (HTTP 403)")
    );
}

#[test]
fn a_secondary_rate_limit_fails_my_prs_as_rate_limited_with_its_own_words() {
    let app = my_prs_failed_by(
        "echo 'gh: You have exceeded a secondary rate limit. Please wait a few minutes before you try again. (HTTP 403)' >&2; exit 1",
    );

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::RateLimited);
    assert_eq!(
        app.focused_error(),
        Some(
            "gh: You have exceeded a secondary rate limit. Please wait a few minutes before you try again. (HTTP 403)"
        )
    );
}

#[test]
fn an_unknown_host_fails_my_prs_as_offline_with_its_own_words() {
    let app = my_prs_failed_by(
        "echo 'error connecting to nosuchhost.invalid' >&2; \
         echo 'check your internet connection or https://githubstatus.com' >&2; exit 1",
    );

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::Offline);
    assert_eq!(
        app.focused_error(),
        Some(
            "error connecting to nosuchhost.invalid\n\
             check your internet connection or https://githubstatus.com"
        )
    );
}

#[test]
fn a_refused_connection_fails_my_prs_as_offline_with_its_own_words() {
    let app = my_prs_failed_by(
        "echo 'Post \"https://api.github.com/graphql\": proxyconnect tcp: dial tcp 127.0.0.1:1: connect: connection refused' >&2; exit 1",
    );

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::Offline);
    assert_eq!(
        app.focused_error(),
        Some(
            "Post \"https://api.github.com/graphql\": proxyconnect tcp: dial tcp 127.0.0.1:1: connect: connection refused"
        )
    );
}

#[test]
fn an_unrecognised_gh_failure_fails_my_prs_as_other_with_its_own_words() {
    let app = my_prs_failed_by("echo 'GraphQL: Something went wrong' >&2; exit 1");

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::Other);
    assert_eq!(app.focused_error(), Some("GraphQL: Something went wrong"));
}

#[test]
fn a_missing_gh_fails_my_prs_as_program_missing_with_its_own_words() {
    let mut app = App::new(Config::default());

    app.receive(
        Stage::Prs,
        0,
        fetched_prs(CliGh::with_program("/nonexistent/gh".into())),
    );

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::ProgramMissing);
    assert_eq!(app.focused_error(), Some(MISSING));
}

#[test]
fn a_signed_out_gh_fails_the_merge_queue_as_not_signed_in_with_its_own_words() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, SIGNED_OUT);
    let repo = acme_api();
    let mut app = App::new(Config::default());

    app.receive(
        Stage::Queue,
        0,
        fetched(Stage::Queue, move |_| {
            fetch_queue(&gh, &repo).map(Rows::Queue)
        }),
    );

    let failure = app.queue.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(failure.to_string(), SIGNED_OUT_SAID);
}

#[test]
fn a_signed_out_gh_fails_main_builds_as_not_signed_in_with_its_own_words() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, SIGNED_OUT);
    let mut source = BuildsSource::new(acme_api());
    let mut app = App::new(Config::default());

    app.receive(
        Stage::Builds,
        0,
        fetched(Stage::Builds, move |_| {
            let builds = source.fetch(&gh, SystemTime::now())?;
            Ok(Rows::Builds(source.into_builds(builds)))
        }),
    );

    let failure = app.builds.active().column.error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(failure.to_string(), SIGNED_OUT_SAID);
}

#[test]
fn a_signed_out_default_branch_lookup_keeps_its_kind_and_names_the_lookup() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, SIGNED_OUT);
    let mut source = BuildsSource::new(Repo {
        main_workflow: "ci.yml".to_string(),
        ..acme_api()
    });

    let failure = source.fetch(&gh, SystemTime::now()).unwrap_err();

    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(
        failure.to_string(),
        format!(
            "reading the default branch of acme/api: {SIGNED_OUT_SAID}; set branch under its [[repo]]"
        )
    );
}

#[test]
fn a_signed_out_gh_without_a_configured_repo_is_not_signed_in_and_says_why() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();

    let failure = repos(&shim(&dir, SIGNED_OUT), &Config::default()).unwrap_err();

    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(
        failure.to_string(),
        format!("no [[repo]] configured and the current directory has none: {SIGNED_OUT_SAID}")
    );
}

fn acme_api() -> Repo {
    Repo {
        name: "acme/api".to_string(),
        ..Repo::default()
    }
}

fn fetched(
    stage: Stage,
    fetch: impl FnMut(usize) -> Result<Rows, Failure> + Send + 'static,
) -> Result<Rows, Failure> {
    let (tx, rx) = mpsc::channel();
    let _refresh = refresh::spawn(stage, Duration::from_secs(60), 1, tx, fetch);
    loop {
        if let Input::Data(_, _, result) = rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap()
        {
            return result;
        }
    }
}

fn my_prs_failed_by(script: &str) -> App {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Config::default());
    app.receive(Stage::Prs, 0, fetched_prs(shim(&dir, script)));
    app
}

fn fetched_prs(gh: CliGh) -> Result<Rows, Failure> {
    let config = Config::default();
    fetched(Stage::Prs, move |tab| {
        fetch_prs(&gh, &config, tab).map(Rows::Prs)
    })
}
