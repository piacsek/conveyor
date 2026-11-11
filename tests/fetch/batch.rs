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
        runs_route(&[(FIRST, "Fix the export button (#4825)")]),
        (
            "repos/acme/webapp/pulls/4825".to_string(),
            serde_json::json!({
                "number": 4825, "title": "Fix the export button",
                "user": {"login": "dana"},
                "html_url": "https://github.com/acme/webapp/pull/4825",
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
    assert_eq!(pull.number, 4825);
    assert_eq!(lookups(&gh).len(), 1);
    assert_eq!(lookups(&gh)[0], "GET repos/acme/webapp/pulls/4825");
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
    assert_eq!(lookups(&gh).len(), 1);
    assert_eq!(
        lookups(&gh)[0],
        format!("GET repos/acme/webapp/commits/{FIRST}")
    );

    source
        .fetch(&gh, NOW + std::time::Duration::from_secs(121))
        .unwrap();

    assert_eq!(batches(&gh).len(), 2);
    assert_eq!(lookups(&gh).len(), 2);
    assert_eq!(
        lookups(&gh)[1],
        format!("GET repos/acme/webapp/commits/{FIRST}")
    );
}

#[test]
fn a_batch_gh_refuses_makes_no_lookup_per_commit_and_is_asked_again_on_the_next_fetch() {
    let mut gh =
        FakeGithub::with_response(Err(std::io::Error::other("gh: API rate limit exceeded")));
    gh.rest_routes = vec![
        runs_route(&[(FIRST, "Retry hooks"), (SECOND, "Add headers (#4822)")]),
        (
            "repos/acme/webapp/commits/".to_string(),
            serde_json::json!([{
                "number": 4821, "title": "Retry hooks", "user": {"login": "alice"},
                "html_url": "https://github.com/acme/webapp/pull/4821"
            }]),
        ),
    ];
    let mut source = source();

    let first = source.fetch(&gh, NOW).unwrap();

    assert_eq!(first[0].pull, None);
    assert_eq!(first[1].pull, None);
    assert_eq!(lookups(&gh), Vec::<String>::new());

    source.fetch(&gh, NOW).unwrap();

    assert_eq!(batches(&gh).len(), 2);
    assert_eq!(lookups(&gh), Vec::<String>::new());
}

#[test]
fn a_sha_that_is_not_forty_hex_characters_stays_out_of_the_batch_and_is_asked_on_its_own() {
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        runs_route(&[("not\"hex", "Odd run"), (FIRST, "Retry hooks")]),
        (
            "repos/acme/webapp/commits/".to_string(),
            serde_json::json!([{
                "number": 4820, "title": "Odd run", "user": {"login": "carol"},
                "html_url": "https://github.com/acme/webapp/pull/4820"
            }]),
        ),
    ];
    gh.graphql_routes = vec![(
        "associatedPullRequests".to_string(),
        serde_json::json!({"data": {"repository": {
            "c0": commit(FIRST, serde_json::json!([pull(4821, "Retry hooks", "alice", "User")])),
        }}}),
    )];

    let builds = source().fetch(&gh, NOW).unwrap();

    assert_eq!(builds[0].pull.as_ref().map(|pull| pull.number), Some(4820));
    assert_eq!(builds[1].pull.as_ref().map(|pull| pull.number), Some(4821));
    assert_eq!(batches(&gh).len(), 1);
    assert!(!batches(&gh)[0].contains("not"));
    assert_eq!(lookups(&gh).len(), 1);
    assert_eq!(
        lookups(&gh)[0],
        "GET repos/acme/webapp/commits/not\"hex/pulls"
    );
}

#[test]
fn a_commit_the_batch_does_not_answer_for_is_asked_on_its_own() {
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        runs_route(&[(FIRST, "Unknown to GitHub"), (SECOND, "Not a commit")]),
        (
            "repos/acme/webapp/commits/".to_string(),
            serde_json::json!([{
                "number": 4820, "title": "Asked on its own", "user": {"login": "carol"},
                "html_url": "https://github.com/acme/webapp/pull/4820"
            }]),
        ),
    ];
    gh.graphql_routes = vec![(
        "associatedPullRequests".to_string(),
        serde_json::json!({"data": {"repository": {"c0": null, "c1": {}}}}),
    )];

    let builds = source().fetch(&gh, NOW).unwrap();

    assert_eq!(builds[0].pull.as_ref().map(|pull| pull.number), Some(4820));
    assert_eq!(builds[1].pull.as_ref().map(|pull| pull.number), Some(4820));
    assert_eq!(lookups(&gh).len(), 2);
    assert_eq!(
        lookups(&gh)[0],
        format!("GET repos/acme/webapp/commits/{FIRST}/pulls")
    );
    assert_eq!(
        lookups(&gh)[1],
        format!("GET repos/acme/webapp/commits/{SECOND}/pulls")
    );
}

#[test]
fn deployed_asks_for_its_environments_pull_requests_in_one_call() {
    use crate::fakes::{FakeKube, deployment_json};
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.graphql_routes = vec![(
        "associatedPullRequests".to_string(),
        serde_json::json!({"data": {"repository": {
            "c0": commit(FIRST, serde_json::json!([pull(4821, "Retry hooks", "alice", "User")])),
            "c1": commit(SECOND, serde_json::json!([pull(4822, "Add headers", "bob", "User")])),
        }}}),
    )];
    let image = |sha: &str| Ok(deployment_json(&format!("ghcr.io/acme/api:{sha}")));
    let kube = FakeKube::with(vec![
        ("dev".to_string(), image(FIRST)),
        ("staging".to_string(), image(FIRST)),
        ("prod".to_string(), image(SECOND)),
    ]);
    let env = |name: &str| DeployEnv {
        name: name.to_string(),
        ..DeployEnv::default()
    };
    let deploy = Deploy {
        env: vec![env("dev"), env("staging"), env("prod")],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    let deployed = source.fetch(&gh, &kube, NOW);

    assert_eq!(
        deployed.rows[0].pull.as_ref().map(|pull| pull.number),
        Some(4821)
    );
    assert_eq!(
        deployed.rows[1].pull.as_ref().map(|pull| pull.number),
        Some(4821)
    );
    assert_eq!(
        deployed.rows[2].pull.as_ref().map(|pull| pull.number),
        Some(4822)
    );
    assert_eq!(batches(&gh).len(), 1);
    assert_eq!(batches(&gh)[0].matches(FIRST).count(), 1);
    assert_eq!(lookups(&gh), Vec::<String>::new());
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
