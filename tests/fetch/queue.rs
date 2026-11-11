use std::io;

use conveyor::sources::github::Variable;

use crate::fakes::FakeGithub;

#[test]
fn fetch_queue_asks_for_the_repo_and_labels_the_result_with_it() {
    use conveyor::config::Repo;
    use conveyor::sources::fetch::fetch_queue;
    let gh = FakeGithub::with_response(Ok(serde_json::from_str(include_str!(
        "../fixtures/queue.json"
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
        "../fixtures/queue.json"
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
