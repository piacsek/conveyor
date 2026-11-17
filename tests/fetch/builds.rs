use crate::fakes::{FakeGithub, NOW};

#[test]
fn builds_source_resolves_the_workflow_by_name_then_lists_main_runs_and_their_pull_requests() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/actions/workflows?".to_string(),
            serde_json::json!({"workflows": [
                {"id": 11, "name": "release", "path": ".github/workflows/release.yml"},
                {"id": 22, "name": "CI/CD", "path": ".github/workflows/cicd.yml"}
            ]}),
        ),
        (
            "repos/acme/webapp/actions/workflows/22/runs".to_string(),
            serde_json::from_str(include_str!("../fixtures/runs.json")).unwrap(),
        ),
        (
            "repos/acme/webapp/commits/b0b51365".to_string(),
            serde_json::from_str(include_str!("../fixtures/commit-pulls.json")).unwrap(),
        ),
        (
            "repos/acme/webapp/commits/".to_string(),
            serde_json::json!([]),
        ),
    ];
    let repo = Repo {
        name: "acme/webapp".to_string(),
        branch: Some("main".to_string()),
        builds: 3,
        ..Repo::default()
    };
    let mut source = BuildsSource::new(repo);

    let builds = source.fetch(&gh, NOW).unwrap();

    assert_eq!(
        builds.len(),
        4,
        "the API decides the page size; per_page carries the config"
    );
    assert_eq!(builds[0].pull.as_ref().map(|p| p.number), Some(3));
    assert_eq!(
        builds[0].pull.as_ref().map(|p| p.author.as_str()),
        Some("piacsek")
    );
    assert_eq!(builds[1].pull, None);
    {
        let calls = gh.calls.borrow().clone();
        let paths: Vec<&str> = calls.iter().map(|(q, _)| q.as_str()).collect();
        assert!(
            paths.contains(&"GET repos/acme/webapp/actions/workflows?per_page=100"),
            "{paths:?}"
        );
        assert!(
            paths.contains(
                &"GET repos/acme/webapp/actions/workflows/22/runs?branch=main&event=push&per_page=3"
            ),
            "{paths:?}"
        );
        assert_eq!(
            paths.iter().filter(|p| p.ends_with("/pulls")).count(),
            4,
            "one association lookup per run: {paths:?}"
        );
        assert_eq!(
            paths
                .iter()
                .filter(|p| p.contains("/commits/") && !p.ends_with("/pulls"))
                .count(),
            3,
            "the three rebase-merged runs carry no (#N), so they read the commit: {paths:?}"
        );
    }

    source.fetch(&gh, NOW).unwrap();
    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls
            .iter()
            .filter(|(q, _)| q.contains("actions/workflows?"))
            .count(),
        1,
        "workflow id is cached"
    );
    assert_eq!(
        calls.iter().filter(|(q, _)| q.ends_with("/pulls")).count(),
        4,
        "a second fetch inside the retry window asks nothing: hits and misses are both cached"
    );
}

#[test]
fn builds_source_uses_a_workflow_file_name_directly_and_reports_unknown_names() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/actions/workflows/ci.yml/runs".to_string(),
            serde_json::json!({"workflow_runs": []}),
        ),
        (
            "repos/acme/webapp/actions/workflows?".to_string(),
            serde_json::json!({"workflows": []}),
        ),
    ];
    let mut by_file = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        branch: Some("main".to_string()),
        ..Repo::default()
    });
    assert_eq!(by_file.fetch(&gh, NOW).unwrap(), vec![]);

    let mut unknown = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "Nope".to_string(),
        branch: Some("main".to_string()),
        ..Repo::default()
    });
    let err = unknown.fetch(&gh, NOW).unwrap_err().to_string();
    assert!(
        err.contains("Nope") && err.contains("main_workflow"),
        "{err}"
    );
}

#[test]
fn the_default_workflow_name_falls_back_to_common_ci_workflows() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/actions/workflows?".to_string(),
            serde_json::json!({"workflows": [
                {"id": 5, "name": "release", "path": ".github/workflows/release.yml"},
                {"id": 7, "name": "ci", "path": ".github/workflows/ci.yml"}
            ]}),
        ),
        (
            "repos/acme/webapp/actions/workflows/7/runs".to_string(),
            serde_json::json!({"workflow_runs": []}),
        ),
    ];
    let mut source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        branch: Some("main".to_string()),
        ..Repo::default()
    });

    assert_eq!(source.fetch(&gh, NOW).unwrap(), vec![]);
    let calls = gh.calls.borrow().clone();
    assert!(
        calls.iter().any(|(q, _)| q.contains("workflows/7/runs")),
        "{calls:?}"
    );
}

#[test]
fn a_configured_branch_names_the_branch_of_the_main_builds_query() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    let mut source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        branch: Some("develop".to_string()),
        builds: 3,
        ..Repo::default()
    });

    source.fetch(&gh, NOW).unwrap();

    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls[0].0,
        "GET repos/acme/webapp/actions/workflows/ci.yml/runs?branch=develop&event=push&per_page=3"
    );
    assert_eq!(calls.len(), 1, "no default branch lookup: {calls:?}");
}

#[test]
fn without_a_configured_branch_main_builds_reads_the_default_branch_once() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/actions/".to_string(),
            serde_json::json!({"workflow_runs": []}),
        ),
        (
            "repos/acme/webapp".to_string(),
            serde_json::json!({"name": "webapp", "default_branch": "trunk"}),
        ),
    ];
    let mut source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        builds: 3,
        ..Repo::default()
    });

    source.fetch(&gh, NOW).unwrap();
    source.fetch(&gh, NOW).unwrap();

    let calls = gh.calls.borrow().clone();
    assert_eq!(calls.len(), 3, "{calls:?}");
    assert_eq!(calls[0].0, "GET repos/acme/webapp");
    assert_eq!(
        calls[1].0,
        "GET repos/acme/webapp/actions/workflows/ci.yml/runs?branch=trunk&event=push&per_page=3"
    );
    assert_eq!(calls[2].0, calls[1].0);
}

#[test]
fn a_failed_default_branch_lookup_is_the_column_error_and_is_asked_again() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_response = Err(std::io::Error::other("gh: Not Found (HTTP 404)"));
    let mut source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        ..Repo::default()
    });

    let first = source.fetch(&gh, NOW).unwrap_err().to_string();
    source.fetch(&gh, NOW).unwrap_err();

    assert_eq!(
        first,
        "reading the default branch of acme/webapp: gh: Not Found (HTTP 404); set branch under its [[repo]]"
    );
    let calls = gh.calls.borrow().clone();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(calls[1].0, "GET repos/acme/webapp");
}

#[test]
fn a_repository_that_names_no_default_branch_is_the_column_error() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_response = Ok(serde_json::json!({"name": "webapp"}));
    let mut source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        ..Repo::default()
    });

    let err = source.fetch(&gh, NOW).unwrap_err().to_string();

    assert_eq!(
        err,
        "acme/webapp answered no default branch; set branch under its [[repo]]"
    );
}

#[test]
fn a_branch_is_percent_encoded_in_the_main_builds_query() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    let mut source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        branch: Some("release/1.x+lts&#2".to_string()),
        builds: 3,
        ..Repo::default()
    });

    source.fetch(&gh, NOW).unwrap();

    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls[0].0,
        "GET repos/acme/webapp/actions/workflows/ci.yml/runs?branch=release/1.x%2Blts%26%232&event=push&per_page=3"
    );
}

#[test]
fn an_empty_main_builds_column_names_the_configured_branch() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    let source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        branch: Some("devlop".to_string()),
        ..Repo::default()
    });

    let screen = screen_after_one_fetch(source, &gh);

    assert!(screen.contains("no builds on devlop"), "{screen}");
}

#[test]
fn an_empty_main_builds_column_names_the_default_branch_it_looked_up() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/actions/".to_string(),
            serde_json::json!({"workflow_runs": []}),
        ),
        (
            "repos/acme/webapp".to_string(),
            serde_json::json!({"default_branch": "trunk"}),
        ),
    ];
    let source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        ..Repo::default()
    });

    let screen = screen_after_one_fetch(source, &gh);

    assert!(screen.contains("no builds on trunk"), "{screen}");
}

fn screen_after_one_fetch(
    mut source: conveyor::sources::fetch::BuildsSource,
    gh: &FakeGithub,
) -> String {
    use conveyor::app::{Input, Rows, Stage};
    let builds = source.fetch(gh, NOW).unwrap();
    let rows = Rows::Builds(source.into_builds(builds));
    let mut harness = crate::support::Harness::new();
    harness
        .run(vec![Ok(Input::Data(Stage::Builds, 0, Ok(rows)))])
        .unwrap();
    harness.screen()
}
