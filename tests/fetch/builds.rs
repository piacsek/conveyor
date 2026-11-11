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
        ..Repo::default()
    });
    assert_eq!(by_file.fetch(&gh, NOW).unwrap(), vec![]);

    let mut unknown = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "Nope".to_string(),
        ..Repo::default()
    });
    let err = unknown.fetch(&gh, NOW).unwrap_err();
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
        ..Repo::default()
    });

    assert_eq!(source.fetch(&gh, NOW).unwrap(), vec![]);
    let calls = gh.calls.borrow().clone();
    assert!(
        calls.iter().any(|(q, _)| q.contains("workflows/7/runs")),
        "{calls:?}"
    );
}
