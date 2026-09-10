use std::cell::RefCell;
use std::io;

use conveyor::config::Config;
use conveyor::fetch::fetch_prs;
use conveyor::github::{Github, Variable};

type Call = (String, Vec<(String, Variable)>);

struct FakeGithub {
    calls: RefCell<Vec<Call>>,
    response: io::Result<serde_json::Value>,
    rest_response: io::Result<serde_json::Value>,
    rest_routes: Vec<(String, serde_json::Value)>,
    cwd_repo: io::Result<String>,
}

impl FakeGithub {
    fn with_response(response: io::Result<serde_json::Value>) -> Self {
        Self {
            calls: RefCell::default(),
            response,
            rest_response: Ok(serde_json::json!({"workflow_runs": []})),
            rest_routes: Vec::new(),
            cwd_repo: Ok("acme/from-cwd".to_string()),
        }
    }
}

impl Github for FakeGithub {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> io::Result<serde_json::Value> {
        self.calls.borrow_mut().push((
            query.to_string(),
            vars.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        match &self.response {
            Ok(value) => Ok(value.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }

    fn rest(&self, path: &str) -> io::Result<serde_json::Value> {
        self.calls
            .borrow_mut()
            .push((format!("GET {path}"), Vec::new()));
        if let Some((_, value)) = self
            .rest_routes
            .iter()
            .find(|(prefix, _)| path.starts_with(prefix))
        {
            return Ok(value.clone());
        }
        match &self.rest_response {
            Ok(value) => Ok(value.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }

    fn current_repo(&self) -> io::Result<String> {
        match &self.cwd_repo {
            Ok(name) => Ok(name.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }
}

#[test]
fn fetch_prs_runs_the_configured_search_and_parses_the_rows() {
    let gh = FakeGithub::with_response(Ok(
        serde_json::from_str(include_str!("fixtures/prs.json")).unwrap()
    ));
    let mut config = Config::default();
    config.prs.query = "is:pr involves:@me".to_string();
    config.prs.limit = 7;

    let prs = fetch_prs(&gh, &config).unwrap();

    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].number, 1);
    let calls = gh.calls.borrow();
    assert!(
        calls[0]
            .0
            .contains("search(query: $q, type: ISSUE, first: $first)")
    );
    assert_eq!(
        calls[0].1,
        vec![
            ("q".to_string(), Variable::from("is:pr involves:@me")),
            ("first".to_string(), Variable::from(7usize)),
        ]
    );
}

#[test]
fn fetch_prs_turns_gh_errors_into_a_message() {
    let gh = FakeGithub::with_response(Err(io::Error::other("gh: HTTP 401: Bad credentials")));

    let err = fetch_prs(&gh, &Config::default()).unwrap_err();

    assert_eq!(err, "gh: HTTP 401: Bad credentials");
}

#[test]
fn repos_come_from_the_config_or_else_from_the_current_directory() {
    use conveyor::config::Repo;
    use conveyor::fetch::repos;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));

    let mut config = Config::default();
    config.repo.push(Repo {
        name: "acme/webapp".to_string(),
        ..Repo::default()
    });
    assert_eq!(
        repos(&gh, &config)
            .unwrap()
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        vec!["acme/webapp"]
    );

    let fallback = repos(&gh, &Config::default()).unwrap();
    assert_eq!(fallback.len(), 1);
    assert_eq!(fallback[0].name, "acme/from-cwd");
    assert_eq!(fallback[0].main_workflow, "CI/CD");

    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.cwd_repo = Err(io::Error::other("not a git repository"));
    let err = repos(&gh, &Config::default()).unwrap_err();
    assert!(err.contains("not a git repository"), "{err}");
    assert!(err.contains("[[repo]]"), "hints at the config: {err}");
}

#[test]
fn fetch_queue_asks_for_the_repo_and_labels_the_result_with_it() {
    use conveyor::config::Repo;
    use conveyor::fetch::fetch_queue;
    let gh = FakeGithub::with_response(Ok(serde_json::from_str(include_str!(
        "fixtures/queue.json"
    ))
    .unwrap()));
    let repo = Repo {
        name: "acme/webapp".to_string(),
        ..Repo::default()
    };

    let queue = fetch_queue(&gh, &repo).unwrap();

    assert_eq!(queue.repo, "acme/webapp");
    assert_eq!(queue.entries.len(), 2);
    let calls = gh.calls.borrow();
    assert!(calls[0].0.contains("mergeQueue"));
    assert_eq!(
        calls[0].1,
        vec![
            ("owner".to_string(), Variable::from("acme")),
            ("name".to_string(), Variable::from("webapp")),
            ("first".to_string(), Variable::from(20usize)),
        ]
    );

    let err = fetch_queue(
        &gh,
        &Repo {
            name: "nope".to_string(),
            ..Repo::default()
        },
    )
    .unwrap_err();
    assert!(err.contains("owner/name"), "{err}");
}

#[test]
fn fetch_queue_attaches_the_merge_group_run_of_each_entry() {
    use conveyor::config::Repo;
    use conveyor::fetch::fetch_queue;
    use conveyor::model::prs::CheckState;
    let mut gh = FakeGithub::with_response(Ok(serde_json::from_str(include_str!(
        "fixtures/queue.json"
    ))
    .unwrap()));
    gh.rest_response = Ok(serde_json::json!({"workflow_runs": [
        {"id": 1, "name": "CI/CD", "status": "in_progress", "conclusion": null,
         "head_branch": "gh-readonly-queue/main/pr-4821-0000000000000000000000000000000000000000",
         "html_url": "https://github.com/acme/webapp/actions/runs/1"},
        {"id": 2, "name": "CI/CD", "status": "completed", "conclusion": "failure",
         "head_branch": "gh-readonly-queue/main/pr-4821-1111111111111111111111111111111111111111",
         "html_url": "https://github.com/acme/webapp/actions/runs/2"},
        {"id": 3, "name": "CI/CD", "status": "completed", "conclusion": "success",
         "head_branch": "gh-readonly-queue/main/pr-9999-1111111111111111111111111111111111111111",
         "html_url": "https://github.com/acme/webapp/actions/runs/3"}
    ]}));
    let repo = Repo {
        name: "acme/webapp".to_string(),
        ..Repo::default()
    };

    let queue = fetch_queue(&gh, &repo).unwrap();

    assert_eq!(
        queue.entries[0].run_url.as_deref(),
        Some("https://github.com/acme/webapp/actions/runs/1"),
        "newest run for the PR wins"
    );
    assert_eq!(queue.entries[0].checks, CheckState::Pending);
    assert_eq!(queue.entries[1].run_url, None);
    let calls = gh.calls.borrow().clone();
    assert!(
        calls
            .iter()
            .any(|(q, _)| q == "GET repos/acme/webapp/actions/runs?event=merge_group&per_page=30"),
        "{calls:?}"
    );

    gh.rest_response = Err(io::Error::other("gh: HTTP 403"));
    let queue = fetch_queue(&gh, &repo).unwrap();
    assert_eq!(
        queue.entries.len(),
        2,
        "runs are optional: the queue still shows"
    );
}

#[test]
fn builds_source_resolves_the_workflow_by_name_then_lists_main_runs_and_their_pull_requests() {
    use conveyor::config::Repo;
    use conveyor::fetch::BuildsSource;
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
            serde_json::from_str(include_str!("fixtures/runs.json")).unwrap(),
        ),
        (
            "repos/acme/webapp/commits/b0b51365".to_string(),
            serde_json::from_str(include_str!("fixtures/commit-pulls.json")).unwrap(),
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

    let builds = source.fetch(&gh).unwrap();

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
        assert_eq!(paths.iter().filter(|p| p.contains("/commits/")).count(), 4);
    }

    source.fetch(&gh).unwrap();
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
        calls
            .iter()
            .filter(|(q, _)| q.contains("/commits/"))
            .count(),
        4,
        "sha lookups are cached"
    );
}

#[test]
fn builds_source_uses_a_workflow_file_name_directly_and_reports_unknown_names() {
    use conveyor::config::Repo;
    use conveyor::fetch::BuildsSource;
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
    assert_eq!(by_file.fetch(&gh).unwrap(), vec![]);

    let mut unknown = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "Nope".to_string(),
        ..Repo::default()
    });
    let err = unknown.fetch(&gh).unwrap_err();
    assert!(
        err.contains("Nope") && err.contains("main_workflow"),
        "{err}"
    );
}
