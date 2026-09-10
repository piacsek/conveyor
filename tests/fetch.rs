use std::cell::RefCell;
use std::io;

use conveyor::config::Config;
use conveyor::sources::fetch::fetch_prs;
use conveyor::sources::github::{Github, Variable};

const NOW: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

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
    images: Vec<(String, io::Result<String>)>,
}

impl FakeKube {
    fn returning(image: &str) -> Self {
        Self {
            images: vec![("prod".to_string(), Ok(image.to_string()))],
        }
    }
}

impl conveyor::sources::kube::Kube for FakeKube {
    fn image(&self, env: &conveyor::config::DeployEnv) -> io::Result<String> {
        match self.images.iter().find(|(name, _)| *name == env.name) {
            Some((_, Ok(image))) => Ok(image.clone()),
            Some((_, Err(err))) => Err(io::Error::new(err.kind(), err.to_string())),
            None => Err(io::Error::other("no such env")),
        }
    }
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
    let kube = FakeKube {
        images: vec![
            (
                "staging".to_string(),
                Ok("ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567".to_string()),
            ),
            (
                "prod".to_string(),
                Err(io::Error::other("ERROR: Active profile expired.")),
            ),
            ("dev".to_string(), Ok("ghcr.io/acme/api:latest".to_string())),
        ],
    };
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
