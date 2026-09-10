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
    cwd_repo: io::Result<String>,
}

impl FakeGithub {
    fn with_response(response: io::Result<serde_json::Value>) -> Self {
        Self {
            calls: RefCell::default(),
            response,
            rest_response: Ok(serde_json::json!({"workflow_runs": []})),
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
