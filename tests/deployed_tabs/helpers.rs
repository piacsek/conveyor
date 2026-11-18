use conveyor::config::{Config, Deploy, DeployEnv, Repo};
use ratatui::crossterm::event::KeyCode;

use crate::support::key;

pub(crate) fn two_systems() -> Config {
    let mut webapp = repo("acme/webapp");
    webapp.deploy = vec![system("api"), system("worker")];
    Config {
        repo: vec![webapp, repo("acme/infra")],
        ..Config::default()
    }
}

pub(crate) fn repo(name: &str) -> Repo {
    Repo {
        name: name.to_string(),
        ..Repo::default()
    }
}

pub(crate) fn system(name: &str) -> Deploy {
    Deploy {
        system: name.to_string(),
        env: vec![DeployEnv {
            name: "prod".to_string(),
            deployment: name.to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    }
}

pub(crate) fn to_deployed() -> Vec<std::io::Result<conveyor::app::Input>> {
    vec![
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ]
}

pub(crate) fn typed_filter(query: &str) -> Vec<std::io::Result<conveyor::app::Input>> {
    let mut keys = vec![key(KeyCode::Char('/'))];
    keys.extend(query.chars().map(|c| key(KeyCode::Char(c))));
    keys.push(key(KeyCode::Enter));
    keys
}
