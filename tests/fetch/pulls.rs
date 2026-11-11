use std::io;

use crate::fakes::{FakeGithub, FakeKube, NOW, SHA, deploy_source_for};

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
