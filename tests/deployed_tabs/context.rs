use conveyor::config::Config;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{repo, system, to_deployed};
use crate::support::{Harness, build, builds_in, deployed_in, deployment, key};

#[test]
fn behind_main_counts_the_builds_of_the_shown_tabs_repository() {
    let mut h = Harness::with_config(160, 24, two_deploying_repos());
    h.run(fetched(deployment("prod", 9, None))).unwrap();
    h.run(to_deployed()).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("↓2"), "{screen}");
}

#[test]
fn b_opens_the_main_build_of_the_shown_tabs_repository() {
    let mut h = Harness::with_config(160, 24, two_deploying_repos());
    h.run(fetched(deployment("prod", 9, None))).unwrap();
    h.run(to_deployed()).unwrap();

    h.run(vec![key(KeyCode::Tab), key(KeyCode::Char('b'))])
        .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/infra/actions/runs/9".to_string()]
    );
}

#[test]
fn b_on_a_sha_older_than_the_builds_opens_the_commit_in_the_shown_tabs_repository() {
    let mut h = Harness::with_config(160, 24, two_deploying_repos());
    h.run(fetched(deployment("prod", 77, None))).unwrap();
    h.run(to_deployed()).unwrap();

    h.run(vec![key(KeyCode::Tab), key(KeyCode::Char('b'))])
        .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec![format!("https://github.com/acme/infra/commit/{:040x}", 77)]
    );
}

fn two_deploying_repos() -> Config {
    let mut webapp = repo("acme/webapp");
    webapp.deploy = vec![system("api")];
    let mut infra = repo("acme/infra");
    infra.deploy = vec![system("edge")];
    Config {
        repo: vec![webapp, infra],
        ..Config::default()
    }
}

fn fetched(
    edge: conveyor::model::deployed::Deployment,
) -> Vec<std::io::Result<conveyor::app::Input>> {
    let mut infra: Vec<_> = [11, 10, 9]
        .into_iter()
        .map(|run| build(run, BuildStatus::Success, None))
        .collect();
    for run in &mut infra {
        run.url = format!(
            "https://github.com/acme/infra/actions/runs/{}",
            run.run_number
        );
    }
    vec![
        builds_in(
            0,
            "acme/webapp",
            vec![
                build(26, BuildStatus::Success, None),
                build(9, BuildStatus::Success, None),
            ],
        ),
        builds_in(1, "acme/infra", infra),
        deployed_in(0, "api", vec![deployment("prod", 26, None)]),
        deployed_in(1, "edge", vec![edge]),
    ]
}
