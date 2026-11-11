use std::io;

use conveyor::config::Config;
use conveyor::sources::fetch::fetch_prs;
use conveyor::sources::github::Variable;

use crate::fakes::FakeGithub;

#[test]
fn fetch_prs_runs_the_configured_search_and_parses_the_rows() {
    let gh = FakeGithub::with_response(Ok(serde_json::from_str(include_str!(
        "../fixtures/prs.json"
    ))
    .unwrap()));
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
fn fetch_prs_asks_which_workflow_run_each_check_came_from() {
    let gh = FakeGithub::with_response(Ok(serde_json::from_str(include_str!(
        "../fixtures/prs.json"
    ))
    .unwrap()));

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
