use conveyor::config::{Config, Deploy, Repo};
use conveyor::model::builds::BuildStatus;
use conveyor::model::queue::QueueEntry;
use ratatui::crossterm::event::KeyCode;

use crate::support::{build, builds_in, entry, key};

pub(crate) fn two_repos() -> Config {
    Config {
        repo: vec![repo("acme/monorepo"), repo("acme/infra")],
        ..Config::default()
    }
}

pub(crate) fn deploy_first() -> Config {
    let mut monorepo = repo("acme/monorepo");
    monorepo.deploy = vec![Deploy {
        system: "api".to_string(),
        ..Deploy::default()
    }];
    Config {
        repo: vec![monorepo, repo("acme/infra")],
        ..Config::default()
    }
}

pub(crate) fn repo(name: &str) -> Repo {
    Repo {
        name: name.to_string(),
        ..Repo::default()
    }
}

pub(crate) fn on_infra_builds() -> Vec<std::io::Result<conveyor::app::Input>> {
    vec![
        builds_in(
            1,
            "acme/infra",
            vec![build(
                25,
                BuildStatus::Failure,
                Some((40, "bob", "Bump nodes")),
            )],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::Char('l')),
    ]
}

pub(crate) fn running(run_number: u64) -> conveyor::model::builds::Build {
    let mut run = build(run_number, BuildStatus::Running, None);
    run.finished_at = None;
    run
}

pub(crate) fn infra_run() -> Vec<QueueEntry> {
    let mut run = build(312, BuildStatus::Failure, None);
    run.id = 4242;
    run.url = "https://github.com/acme/infra/actions/runs/4242".to_string();
    let mut first = entry(1, 40, "bob", "Bump nodes");
    first.run = Some(run);
    vec![first]
}

pub(crate) fn running_infra_run() -> Vec<QueueEntry> {
    let mut entries = infra_run();
    if let Some(run) = entries[0].run.as_mut() {
        run.status = BuildStatus::Running;
    }
    entries
}
