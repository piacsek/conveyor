use std::cell::RefCell;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

use conveyor::config::{Config, MergeMethod, UpdateMethod};
use conveyor::model::prs::{BranchUpdate, StatusChange};
use conveyor::sources::fetch::fetch_prs;
use conveyor::sources::github::{Github, Variable};

const NOW: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

type Call = (String, Vec<(String, Variable)>);

struct FakeGithub {
    calls: RefCell<Vec<Call>>,
    response: io::Result<serde_json::Value>,
    rest_response: io::Result<serde_json::Value>,
    rest_routes: Vec<(String, serde_json::Value)>,
    graphql_routes: Vec<(String, serde_json::Value)>,
    cwd_repo: io::Result<String>,
    log_response: io::Result<String>,
    rerun_failures: Vec<(u64, String)>,
}

impl FakeGithub {
    fn with_response(response: io::Result<serde_json::Value>) -> Self {
        Self {
            calls: RefCell::default(),
            response,
            rest_response: Ok(serde_json::json!({"workflow_runs": []})),
            rest_routes: Vec::new(),
            graphql_routes: Vec::new(),
            cwd_repo: Ok("acme/from-cwd".to_string()),
            log_response: Ok(String::new()),
            rerun_failures: Vec::new(),
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
        if let Some((_, value)) = self
            .graphql_routes
            .iter()
            .find(|(needle, _)| query.contains(needle.as_str()))
        {
            return Ok(value.clone());
        }
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

    fn log_failed(&self, repo: &str, run_id: u64, job_id: u64) -> io::Result<String> {
        self.calls
            .borrow_mut()
            .push((format!("LOG {repo} {run_id} {job_id}"), Vec::new()));
        match &self.log_response {
            Ok(text) => Ok(text.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }

    fn rerun_failed(&self, repo: &str, run_id: u64) -> io::Result<()> {
        self.calls
            .borrow_mut()
            .push((format!("RERUN {repo} {run_id}"), Vec::new()));
        match self.rerun_failures.iter().find(|(id, _)| *id == run_id) {
            Some((_, message)) => Err(io::Error::other(message.clone())),
            None => Ok(()),
        }
    }

    fn merge(
        &self,
        repo: &str,
        number: u64,
        method: MergeMethod,
        head_oid: &str,
    ) -> io::Result<()> {
        self.calls.borrow_mut().push((
            format!("MERGE {repo} {number} {method:?} {head_oid}"),
            Vec::new(),
        ));
        Ok(())
    }

    fn update_branch(
        &self,
        repo: &str,
        number: u64,
        method: UpdateMethod,
    ) -> io::Result<BranchUpdate> {
        self.calls
            .borrow_mut()
            .push((format!("UPDATE {repo} {number} {method:?}"), Vec::new()));
        Ok(BranchUpdate::Updated)
    }

    fn change_status(&self, repo: &str, number: u64, change: &StatusChange) -> io::Result<()> {
        self.calls
            .borrow_mut()
            .push((format!("STATUS {repo} {number} {change:?}"), Vec::new()));
        Ok(())
    }

    fn checkout(&self, repo: &str, number: u64, _dir: Option<&std::path::Path>) -> io::Result<()> {
        self.calls
            .borrow_mut()
            .push((format!("CHECKOUT {repo} {number}"), Vec::new()));
        Ok(())
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
            .contains("search(query: $q, type: ISSUE_ADVANCED, first: $first)")
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
    use conveyor::sources::fetch::repos;
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
    use conveyor::sources::fetch::fetch_queue;
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
    use conveyor::model::prs::CheckState;
    use conveyor::sources::fetch::fetch_queue;
    let mut gh = FakeGithub::with_response(Ok(serde_json::from_str(include_str!(
        "fixtures/queue.json"
    ))
    .unwrap()));
    gh.rest_response = Ok(serde_json::json!({"workflow_runs": [
        {"id": 2, "name": "CI/CD", "status": "in_progress", "conclusion": null,
         "head_branch": "gh-readonly-queue/main/pr-4821-0000000000000000000000000000000000000000",
         "html_url": "https://github.com/acme/webapp/actions/runs/2"},
        {"id": 1, "name": "CI/CD", "status": "completed", "conclusion": "failure",
         "head_branch": "gh-readonly-queue/main/pr-4821-1111111111111111111111111111111111111111",
         "html_url": "https://github.com/acme/webapp/actions/runs/1"},
        {"id": 3, "name": "CI/CD", "status": "completed", "conclusion": "success",
         "head_branch": "gh-readonly-queue/main/pr-9999-1111111111111111111111111111111111111111",
         "html_url": "https://github.com/acme/webapp/actions/runs/3"}
    ]}));
    let repo = Repo {
        name: "acme/webapp".to_string(),
        ..Repo::default()
    };

    let queue = fetch_queue(&gh, &repo).unwrap();

    let run = queue.entries[0]
        .run
        .as_ref()
        .expect("the newest run for the PR wins");
    assert_eq!(run.id, 2, "the higher id is the newer run");
    assert_eq!(run.url, "https://github.com/acme/webapp/actions/runs/2");
    assert_eq!(
        run.status,
        conveyor::model::builds::BuildStatus::Running,
        "the whole run comes along, not just its URL"
    );
    assert_eq!(queue.entries[0].checks, CheckState::Pending);
    assert!(queue.entries[1].run.is_none());
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

struct FakeKube {
    deployments: Vec<(String, io::Result<String>)>,
    pods: Result<String, String>,
    selectors: std::sync::Mutex<Vec<String>>,
}

impl FakeKube {
    fn returning(image: &str) -> Self {
        Self::with(vec![("prod".to_string(), Ok(deployment_json(image)))])
    }

    fn with(deployments: Vec<(String, io::Result<String>)>) -> Self {
        Self {
            deployments,
            pods: Ok(r#"{"items":[]}"#.to_string()),
            selectors: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl conveyor::sources::kube::Kube for FakeKube {
    fn deployment(&self, env: &conveyor::config::DeployEnv) -> io::Result<String> {
        match self.deployments.iter().find(|(name, _)| *name == env.name) {
            Some((_, Ok(json))) => Ok(json.clone()),
            Some((_, Err(err))) => Err(io::Error::new(err.kind(), err.to_string())),
            None => Err(io::Error::other("no such env")),
        }
    }

    fn pods(&self, _env: &conveyor::config::DeployEnv, selector: &str) -> io::Result<String> {
        self.selectors.lock().unwrap().push(selector.to_string());
        self.pods.clone().map_err(io::Error::other)
    }
}

#[test]
fn deploy_source_reads_its_environments_at_the_same_time() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    let kube = OverlappingKube::default();
    let deploy = Deploy {
        env: ["dev", "staging", "prod"]
            .iter()
            .map(|name| DeployEnv {
                name: name.to_string(),
                ..DeployEnv::default()
            })
            .collect(),
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);

    assert_eq!(kube.most_at_once.load(Ordering::SeqCst), 3);
}

#[test]
fn deploy_source_reads_each_environment_and_keeps_per_env_errors_on_the_row() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/commits/0123456789abcdef0123456789abcdef01234567".to_string(),
            serde_json::from_str(include_str!("fixtures/commit-pulls.json")).unwrap(),
        ),
        (
            "repos/acme/webapp/commits/".to_string(),
            serde_json::json!([]),
        ),
    ];
    let kube = FakeKube::with(vec![
        (
            "staging".to_string(),
            Ok(deployment_json(
                "ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567",
            )),
        ),
        (
            "prod".to_string(),
            Err(io::Error::other("ERROR: Active profile expired.")),
        ),
        (
            "dev".to_string(),
            Ok(deployment_json("ghcr.io/acme/api:latest")),
        ),
    ]);
    let deploy = Deploy {
        system: "api".to_string(),
        env: ["staging", "prod", "dev"]
            .iter()
            .map(|name| DeployEnv {
                name: name.to_string(),
                ..DeployEnv::default()
            })
            .collect(),
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    let deployed = source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);

    assert_eq!(deployed.system, "api");
    assert_eq!(deployed.rows.len(), 3);
    assert_eq!(deployed.rows[0].env, "staging");
    assert_eq!(
        deployed.rows[0].sha.as_deref(),
        Some("0123456789abcdef0123456789abcdef01234567")
    );
    assert_eq!(deployed.rows[0].pull.as_ref().map(|p| p.number), Some(3));
    assert_eq!(deployed.rows[0].error, None);
    assert_eq!(
        deployed.rows[1].error.as_deref(),
        Some("ERROR: Active profile expired.")
    );
    assert_eq!(deployed.rows[1].sha, None);
    assert!(
        deployed.rows[2]
            .error
            .as_deref()
            .unwrap()
            .contains("unexpected image tag")
    );
    assert_eq!(
        deployed.rows[2].image.as_deref(),
        Some("ghcr.io/acme/api:latest")
    );

    source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);
    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls
            .iter()
            .filter(|(q, _)| q.contains("/commits/"))
            .count(),
        1,
        "sha lookup cached"
    );
}

#[test]
fn fetch_jobs_reads_the_runs_jobs_route_and_parses_them() {
    use conveyor::sources::fetch::fetch_jobs;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![(
        "repos/acme/webapp/actions/runs/1025/jobs".to_string(),
        serde_json::from_str(include_str!("fixtures/jobs.json")).unwrap(),
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
fn a_missing_pull_request_is_re_asked_once_the_retry_window_passes() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![(
        "repos/acme/webapp/commits/".to_string(),
        serde_json::json!([]),
    )];
    let kube = FakeKube::returning("ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567");
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);
    let associations = |gh: &FakeGithub| {
        gh.calls
            .borrow()
            .iter()
            .filter(|(path, _)| path.ends_with("/pulls"))
            .count()
    };

    for _ in 0..3 {
        assert_eq!(source.fetch(&gh, &kube, NOW).rows[0].pull, None);
    }
    assert_eq!(
        associations(&gh),
        1,
        "a miss is remembered for the retry window"
    );

    source.fetch(&gh, &kube, NOW + std::time::Duration::from_secs(121));
    assert_eq!(
        associations(&gh),
        2,
        "and asked again after it, so the row heals itself without a restart"
    );
}

#[test]
fn an_error_from_gh_is_not_an_answer_and_never_triggers_the_fallback() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let gh = FakeGithub {
        rest_response: Err(io::Error::other("gh: HTTP 403: rate limit exceeded")),
        ..FakeGithub::with_response(Ok(serde_json::Value::Null))
    };
    let kube = FakeKube::returning("ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567");
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    for _ in 0..2 {
        assert_eq!(source.fetch(&gh, &kube, NOW).rows[0].pull, None);
    }

    let calls = gh.calls.borrow().clone();
    assert!(
        calls.iter().all(|(path, _)| path.ends_with("/pulls")),
        "no commit read and no pull read while gh is refusing: {calls:?}"
    );
    assert_eq!(
        calls.len(),
        2,
        "an error is not cached either, so it retries: {calls:?}"
    );
}

#[test]
fn a_pull_lookup_that_succeeds_is_cached() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![(
        "repos/acme/webapp/commits/".to_string(),
        serde_json::json!([{
            "number": 4821, "title": "Retry hooks", "user": {"login": "alice"},
            "html_url": "https://github.com/acme/webapp/pull/4821"
        }]),
    )];
    let kube = FakeKube::returning("ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567");
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    for _ in 0..2 {
        let deployed = source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);
        assert_eq!(
            deployed.rows[0].pull.as_ref().map(|pull| pull.number),
            Some(4821)
        );
    }

    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls
            .iter()
            .filter(|(path, _)| path.contains("/pulls"))
            .count(),
        1,
        "asked once: {calls:?}"
    );
}

#[test]
fn a_commit_with_no_associated_pull_falls_back_to_the_squash_suffix() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let sha = "0123456789abcdef0123456789abcdef01234567";
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            format!("repos/acme/webapp/commits/{sha}/pulls"),
            serde_json::json!([]),
        ),
        (
            format!("repos/acme/webapp/commits/{sha}"),
            serde_json::json!({
                "commit": {"message": "Fix the handbook share (#6882)\n\nlong body"}
            }),
        ),
        (
            "repos/acme/webapp/pulls/6882".to_string(),
            serde_json::json!({
                "number": 6882, "title": "Fix the handbook share",
                "user": {"login": "ailin"},
                "html_url": "https://github.com/acme/webapp/pull/6882",
                "merge_commit_sha": sha
            }),
        ),
    ];
    let kube = FakeKube::returning(&format!("ghcr.io/acme/api:{sha}"));
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    let deployed = source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);

    let pull = deployed.rows[0].pull.as_ref().expect("the squash suffix");
    assert_eq!(pull.number, 6882);
    assert_eq!(pull.title, "Fix the handbook share");
    assert_eq!(pull.author, "ailin");
    assert_eq!(pull.url, "https://github.com/acme/webapp/pull/6882");
}

#[test]
fn deploy_source_reads_the_pods_behind_the_deployment_and_judges_its_health() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::model::deployed::Health;
    use conveyor::sources::fetch::DeploySource;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    let mut kube = FakeKube::with(vec![(
        "prod".to_string(),
        Ok(include_str!("fixtures/deployment.json").to_string()),
    )]);
    kube.pods = Ok(include_str!("fixtures/pods.json").to_string());
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            context: "ctx-prod".to_string(),
            namespace: "web".to_string(),
            deployment: "api".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    let deployed = source.fetch(&gh, &kube, NOW);

    let row = &deployed.rows[0];
    assert_eq!(
        row.sha.as_deref(),
        Some("8c1f2a7d4e9b01c35f6a7b8c9d0e1f2a3b4c5d6e")
    );
    assert_eq!(row.error, None);
    assert_eq!(
        row.health,
        Health::Warning("1 of 3 pods CrashLoopBackOff".to_string())
    );
    assert_eq!(row.pods.len(), 3);
    assert_eq!(row.pods_error, None);
    assert_eq!(row.rollout.as_ref().map(|rollout| rollout.desired), Some(2));
    assert_eq!(row.target, "ctx-prod / web / api");
    assert_eq!(
        *kube.selectors.lock().unwrap(),
        vec!["app.kubernetes.io/instance=api,app.kubernetes.io/name=api"]
    );
}

#[test]
fn pods_that_cannot_be_read_leave_the_row_unread_instead_of_failing_it() {
    use conveyor::model::deployed::Health;
    let (gh, mut kube, mut source) = deploy_source_for(SHA, Vec::new());
    kube.pods = Err("Error from server (Forbidden): pods is forbidden".to_string());

    let deployed = source.fetch(&gh, &kube, NOW);

    let row = &deployed.rows[0];
    assert_eq!(row.error, None);
    assert_eq!(row.sha.as_deref(), Some(SHA));
    assert_eq!(row.health, Health::Unread);
    assert_eq!(
        row.pods_error.as_deref(),
        Some("Error from server (Forbidden): pods is forbidden")
    );
}

#[test]
fn a_deployment_without_match_labels_is_unread_and_asks_for_no_pods() {
    use conveyor::model::deployed::Health;
    let (gh, mut kube, mut source) = deploy_source_for(SHA, Vec::new());
    kube.deployments = vec![(
        "prod".to_string(),
        Ok(serde_json::json!({
            "spec": {"template": {"spec": {"containers": [{"image": format!("ghcr.io/acme/api:{SHA}")}]}}},
            "status": {"conditions": [{"type": "Available", "status": "True"}]}
        })
        .to_string()),
    )];

    let deployed = source.fetch(&gh, &kube, NOW);

    let row = &deployed.rows[0];
    assert_eq!(row.health, Health::Unread);
    assert_eq!(
        row.pods_error.as_deref(),
        Some("the deployment's selector has no matchLabels")
    );
    assert_eq!(kube.selectors.lock().unwrap().len(), 0);
}

fn deploy_source_for(
    sha: &str,
    routes: Vec<(String, serde_json::Value)>,
) -> (FakeGithub, FakeKube, conveyor::sources::fetch::DeploySource) {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = routes;
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    (
        gh,
        FakeKube::returning(&format!("ghcr.io/acme/api:{sha}")),
        DeploySource::new("acme/webapp".to_string(), deploy),
    )
}

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn a_squash_suffix_that_names_a_pull_request_the_commit_is_not_in_is_refused() {
    let (gh, kube, mut source) = deploy_source_for(
        SHA,
        vec![
            (
                format!("repos/acme/webapp/commits/{SHA}/pulls"),
                serde_json::json!([]),
            ),
            (
                format!("repos/acme/webapp/commits/{SHA}"),
                serde_json::json!({"commit": {"message": "Fix the webhook retry (#4821)"}}),
            ),
            (
                "repos/acme/webapp/pulls/4821".to_string(),
                serde_json::json!({
                    "number": 4821, "title": "Fix the webhook retry",
                    "user": {"login": "alice"},
                    "html_url": "https://github.com/acme/webapp/pull/4821",
                    "merge_commit_sha": "9999999999999999999999999999999999999999",
                    "head": {"sha": "8888888888888888888888888888888888888888"}
                }),
            ),
        ],
    );

    let deployed = source.fetch(&gh, &kube, NOW);

    assert_eq!(
        deployed.rows[0].pull, None,
        "a cherry-picked commit keeps the subject of a pull request it is not part of"
    );
}

#[test]
fn a_pull_request_whose_head_is_the_commit_is_accepted() {
    let (gh, kube, mut source) = deploy_source_for(
        SHA,
        vec![
            (
                format!("repos/acme/webapp/commits/{SHA}/pulls"),
                serde_json::json!([]),
            ),
            (
                format!("repos/acme/webapp/commits/{SHA}"),
                serde_json::json!({"commit": {"message": "Fix the webhook retry (#4821)"}}),
            ),
            (
                "repos/acme/webapp/pulls/4821".to_string(),
                serde_json::json!({
                    "number": 4821, "title": "Fix the webhook retry",
                    "user": {"login": "alice"},
                    "html_url": "https://github.com/acme/webapp/pull/4821",
                    "head": {"sha": SHA}
                }),
            ),
        ],
    );

    let deployed = source.fetch(&gh, &kube, NOW);

    assert_eq!(
        deployed.rows[0].pull.as_ref().map(|pull| pull.number),
        Some(4821)
    );
}

#[test]
fn a_commit_message_without_a_suffix_on_its_first_line_resolves_to_nothing() {
    for message in [
        "Add rate limit headers",
        "Add rate limit headers\n\nfixes something (#4821)",
        "Add rate limit headers (#abc)",
        "Add rate limit headers (#)",
    ] {
        let (gh, kube, mut source) = deploy_source_for(
            SHA,
            vec![
                (
                    format!("repos/acme/webapp/commits/{SHA}/pulls"),
                    serde_json::json!([]),
                ),
                (
                    format!("repos/acme/webapp/commits/{SHA}"),
                    serde_json::json!({"commit": {"message": message}}),
                ),
            ],
        );

        let deployed = source.fetch(&gh, &kube, NOW);

        assert_eq!(deployed.rows[0].pull, None, "{message:?}");
        let calls = gh.calls.borrow().clone();
        assert!(
            !calls.iter().any(|(path, _)| path.contains("/pulls/")),
            "no pull request is fetched for {message:?}: {calls:?}"
        );
    }
}

#[test]
fn a_pull_request_read_that_fails_or_arrives_malformed_resolves_to_nothing() {
    for body in [
        serde_json::json!({}),
        serde_json::json!({"merge_commit_sha": SHA}),
    ] {
        let (gh, kube, mut source) = deploy_source_for(
            SHA,
            vec![
                (
                    format!("repos/acme/webapp/commits/{SHA}/pulls"),
                    serde_json::json!([]),
                ),
                (
                    format!("repos/acme/webapp/commits/{SHA}"),
                    serde_json::json!({"commit": {"message": "Fix it (#4821)"}}),
                ),
                ("repos/acme/webapp/pulls/4821".to_string(), body.clone()),
            ],
        );

        let deployed = source.fetch(&gh, &kube, NOW);

        assert_eq!(deployed.rows[0].pull, None, "{body}");
    }
}

#[test]
fn a_run_that_carries_its_own_number_skips_the_commit_read() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::BuildsSource;
    let sha = "b0b513654654b311044cebdd3a61d6ccc436c6fa";
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/actions/workflows/ci.yml/runs".to_string(),
            serde_json::json!({"workflow_runs": [{
                "id": 1, "run_number": 42, "status": "completed", "conclusion": "success",
                "head_sha": sha, "display_title": "Fix the handbook share (#6882)",
                "html_url": "https://github.com/acme/webapp/actions/runs/1",
                "actor": {"login": "merge-bot"},
                "run_started_at": "2026-09-10T17:35:22Z", "updated_at": "2026-09-10T17:35:41Z"
            }]}),
        ),
        (
            format!("repos/acme/webapp/commits/{sha}/pulls"),
            serde_json::json!([]),
        ),
        (
            "repos/acme/webapp/pulls/6882".to_string(),
            serde_json::json!({
                "number": 6882, "title": "Fix the handbook share",
                "user": {"login": "ailin"},
                "html_url": "https://github.com/acme/webapp/pull/6882",
                "merge_commit_sha": sha
            }),
        ),
    ];
    let mut source = BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        ..Repo::default()
    });

    let builds = source.fetch(&gh, NOW).unwrap();

    assert_eq!(
        builds[0].pull.as_ref().map(|pull| pull.author.as_str()),
        Some("ailin"),
        "the author comes from the pull request, not the run actor"
    );
    let calls = gh.calls.borrow().clone();
    assert!(
        !calls
            .iter()
            .any(|(path, _)| path.ends_with(&format!("commits/{sha}"))),
        "the (#N) on the run title is already in hand: {calls:?}"
    );
}

#[test]
fn a_clock_that_steps_backwards_retries_rather_than_freezing_the_row() {
    let (gh, kube, mut source) = deploy_source_for(
        SHA,
        vec![(
            format!("repos/acme/webapp/commits/{SHA}/pulls"),
            serde_json::json!([]),
        )],
    );
    let later = NOW + std::time::Duration::from_secs(600);

    source.fetch(&gh, &kube, later);
    source.fetch(&gh, &kube, NOW);

    assert_eq!(
        gh.calls
            .borrow()
            .iter()
            .filter(|(path, _)| path.ends_with("/pulls"))
            .count(),
        2,
        "an unmeasurable window is not a fresh one: the safe direction is to ask again"
    );
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

#[test]
fn fetch_prs_asks_which_workflow_run_each_check_came_from() {
    let gh = FakeGithub::with_response(Ok(
        serde_json::from_str(include_str!("fixtures/prs.json")).unwrap()
    ));

    fetch_prs(&gh, &Config::default()).unwrap();

    let calls = gh.calls.borrow();
    assert!(
        calls[0].0.contains(
            "databaseId checkSuite { conclusion app { slug } workflowRun { databaseId event workflow { name } } }"
        ),
        "{}",
        calls[0].0
    );
}

#[test]
fn fetch_prs_reads_every_page_of_a_pull_requests_checks() {
    let check = |id: u64, status: &str| {
        serde_json::json!({"name": "test", "status": status, "conclusion": null, "detailsUrl": "",
            "databaseId": id, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "CI"}}}})
    };
    let mut gh = FakeGithub::with_response(Ok(serde_json::json!({"data": {"search": {"nodes": [
        {"id": "PR_1", "number": 1, "statusCheckRollup": {"state": "PENDING", "contexts": {
            "pageInfo": {"hasNextPage": true, "endCursor": "c1"},
            "nodes": [check(1, "COMPLETED")]}}}
    ]}}})));
    gh.graphql_routes = vec![(
        "node(id: $id)".to_string(),
        serde_json::json!({"data": {"node": {"statusCheckRollup": {"contexts": {
            "pageInfo": {"hasNextPage": false, "endCursor": "c2"},
            "nodes": [check(2, "IN_PROGRESS")]}}}}}),
    )];

    let prs = fetch_prs(&gh, &Config::default()).unwrap();

    assert_eq!(prs[0].checks_detail.len(), 1);
    assert_eq!(
        prs[0].checks_detail[0].conclusion,
        conveyor::model::prs::CheckConclusion::Pending
    );
    let calls = gh.calls.borrow();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1].1,
        vec![
            ("id".to_string(), Variable::from("PR_1")),
            ("after".to_string(), Variable::from("c1")),
        ]
    );
}

fn deployment_json(image: &str) -> String {
    serde_json::json!({
        "spec": {
            "replicas": 1,
            "selector": {"matchLabels": {"app": "api"}},
            "template": {"spec": {"containers": [{"image": image}]}}
        },
        "status": {
            "replicas": 1,
            "updatedReplicas": 1,
            "readyReplicas": 1,
            "availableReplicas": 1,
            "conditions": [{"type": "Available", "status": "True"}]
        }
    })
    .to_string()
}

#[derive(Default)]
struct OverlappingKube {
    running: std::sync::Mutex<usize>,
    another_started: std::sync::Condvar,
    most_at_once: AtomicUsize,
}

impl conveyor::sources::kube::Kube for OverlappingKube {
    fn deployment(&self, _env: &conveyor::config::DeployEnv) -> io::Result<String> {
        let mut running = self.running.lock().unwrap();
        *running += 1;
        self.most_at_once.fetch_max(*running, Ordering::SeqCst);
        self.another_started.notify_all();
        let (mut running, _) = self
            .another_started
            .wait_timeout_while(running, std::time::Duration::from_secs(2), |_| {
                self.most_at_once.load(Ordering::SeqCst) < 3
            })
            .unwrap();
        *running -= 1;
        Err(io::Error::other("unreachable"))
    }

    fn pods(&self, _env: &conveyor::config::DeployEnv, _selector: &str) -> io::Result<String> {
        Ok(r#"{"items":[]}"#.to_string())
    }
}
