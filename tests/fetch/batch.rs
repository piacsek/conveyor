use conveyor::config::Repo;
use conveyor::sources::fetch::BuildsSource;

use crate::fakes::{FakeGithub, NOW};

const FIRST: &str = "1111111111aaaaaaaaaa1111111111aaaaaaaaaa";
const SECOND: &str = "2222222222bbbbbbbbbb2222222222bbbbbbbbbb";

#[test]
fn main_builds_asks_for_every_run_s_pull_request_in_one_call() {
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![runs_route(&[
        (FIRST, "Retry hooks"),
        (SECOND, "Add headers"),
    ])];
    gh.graphql_routes = vec![(
        "associatedPullRequests".to_string(),
        serde_json::json!({"data": {"repository": {
            "c0": commit(FIRST, serde_json::json!([pull(4821, "Retry hooks", "alice", "User")])),
            "c1": commit(SECOND, serde_json::json!([pull(4822, "Add headers", "bob", "User")])),
        }}}),
    )];
    let mut source = source();

    let builds = source.fetch(&gh, NOW).unwrap();

    let first = builds[0]
        .pull
        .as_ref()
        .expect("the first run's pull request");
    assert_eq!(first.number, 4821);
    assert_eq!(first.title, "Retry hooks");
    assert_eq!(first.author, "alice");
    assert_eq!(first.url, "https://github.com/acme/webapp/pull/4821");
    let second = builds[1]
        .pull
        .as_ref()
        .expect("the second run's pull request");
    assert_eq!(second.number, 4822);
    assert_eq!(batches(&gh).len(), 1);
    assert!(batches(&gh)[0].contains(FIRST));
    assert!(batches(&gh)[0].contains(SECOND));
    assert_eq!(lookups(&gh), Vec::<String>::new());
}

#[test]
fn a_bot_s_pull_request_keeps_the_bot_suffix_rest_gave_it() {
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![runs_route(&[(FIRST, "Bump serde")])];
    gh.graphql_routes = vec![(
        "associatedPullRequests".to_string(),
        serde_json::json!({"data": {"repository": {
            "c0": commit(FIRST, serde_json::json!([pull(4823, "Bump serde", "dependabot", "Bot")])),
        }}}),
    )];

    let builds = source().fetch(&gh, NOW).unwrap();

    let pull = builds[0].pull.as_ref().expect("the bot's pull request");
    assert_eq!(pull.author, "dependabot[bot]");
}

#[test]
fn a_commit_the_batch_finds_no_pull_request_for_goes_straight_to_the_number_on_its_title() {
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        runs_route(&[(FIRST, "Fix the handbook share (#6882)")]),
        (
            "repos/acme/webapp/pulls/6882".to_string(),
            serde_json::json!({
                "number": 6882, "title": "Fix the handbook share",
                "user": {"login": "ailin"},
                "html_url": "https://github.com/acme/webapp/pull/6882",
                "merge_commit_sha": FIRST
            }),
        ),
    ];
    gh.graphql_routes = vec![(
        "associatedPullRequests".to_string(),
        serde_json::json!({"data": {"repository": {"c0": commit(FIRST, serde_json::json!([]))}}}),
    )];

    let builds = source().fetch(&gh, NOW).unwrap();

    let pull = builds[0]
        .pull
        .as_ref()
        .expect("the pull request on the title");
    assert_eq!(pull.number, 6882);
    assert_eq!(lookups(&gh), vec!["GET repos/acme/webapp/pulls/6882"]);
}

#[test]
fn a_commit_without_a_pull_request_is_asked_again_in_a_batch_once_the_retry_window_passes() {
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        runs_route(&[(FIRST, "Pushed straight to main")]),
        (
            format!("repos/acme/webapp/commits/{FIRST}"),
            serde_json::json!({"commit": {"message": "Pushed straight to main"}}),
        ),
    ];
    gh.graphql_routes = vec![(
        "associatedPullRequests".to_string(),
        serde_json::json!({"data": {"repository": {"c0": commit(FIRST, serde_json::json!([]))}}}),
    )];
    let mut source = source();

    let first = source.fetch(&gh, NOW).unwrap();
    let second = source.fetch(&gh, NOW).unwrap();

    assert_eq!(first[0].pull, None);
    assert_eq!(second[0].pull, None);
    assert_eq!(batches(&gh).len(), 1);
    assert_eq!(
        lookups(&gh),
        vec![format!("GET repos/acme/webapp/commits/{FIRST}")]
    );

    source
        .fetch(&gh, NOW + std::time::Duration::from_secs(121))
        .unwrap();

    assert_eq!(batches(&gh).len(), 2);
    assert_eq!(
        lookups(&gh),
        vec![
            format!("GET repos/acme/webapp/commits/{FIRST}"),
            format!("GET repos/acme/webapp/commits/{FIRST}")
        ]
    );
}

fn source() -> BuildsSource {
    BuildsSource::new(Repo {
        name: "acme/webapp".to_string(),
        main_workflow: "ci.yml".to_string(),
        ..Repo::default()
    })
}

fn runs_route(runs: &[(&str, &str)]) -> (String, serde_json::Value) {
    let runs: Vec<serde_json::Value> = runs
        .iter()
        .enumerate()
        .map(|(index, (sha, title))| {
            serde_json::json!({
                "id": index + 1, "run_number": index + 1, "status": "completed",
                "conclusion": "success", "head_sha": sha, "display_title": title,
                "html_url": format!("https://github.com/acme/webapp/actions/runs/{}", index + 1),
                "actor": {"login": "merge-bot"},
                "run_started_at": "2026-09-10T17:35:22Z", "updated_at": "2026-09-10T17:35:41Z"
            })
        })
        .collect();
    (
        "repos/acme/webapp/actions/workflows/ci.yml/runs".to_string(),
        serde_json::json!({"workflow_runs": runs}),
    )
}

fn commit(sha: &str, pulls: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"oid": sha, "associatedPullRequests": {"nodes": pulls}})
}

fn pull(number: u64, title: &str, login: &str, kind: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number, "title": title,
        "url": format!("https://github.com/acme/webapp/pull/{number}"),
        "author": {"login": login, "__typename": kind}
    })
}

fn batches(gh: &FakeGithub) -> Vec<String> {
    gh.calls
        .borrow()
        .iter()
        .filter(|(query, _)| query.contains("associatedPullRequests"))
        .map(|(query, _)| query.clone())
        .collect()
}

fn lookups(gh: &FakeGithub) -> Vec<String> {
    gh.calls
        .borrow()
        .iter()
        .filter(|(path, _)| path.contains("/commits/") || path.contains("/pulls/"))
        .map(|(path, _)| path.clone())
        .collect()
}
