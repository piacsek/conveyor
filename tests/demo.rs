use std::time::SystemTime;

use conveyor::config::{Config, Prs, Repo, Ui};
use conveyor::sources::demo::{self, Demo};
use conveyor::sources::fetch::{BuildsSource, fetch_jobs, fetch_queue};

#[test]
fn the_demo_watches_its_own_repositories_instead_of_the_users() {
    let user = Config {
        repo: vec![Repo {
            name: "someone/else".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let config = demo::config(user);

    assert_eq!(config.repo.len(), 2);
    assert_eq!(config.repo[0].name, "acme/webapp");
    assert_eq!(config.repo[1].name, "acme/infra");
}

#[test]
fn the_demo_deploys_acme_webapp_to_four_environments() {
    let config = demo::config(Config::default());

    let envs = &config.repo[0].deploy[0].env;
    assert_eq!(envs.len(), 4);
    assert_eq!(envs[0].name, "dev");
    assert_eq!(envs[3].name, "uat");
}

#[test]
fn the_demo_has_its_own_my_prs_tabs() {
    let user = Config {
        prs: Prs {
            limit: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let config = demo::config(user);

    assert_eq!(config.prs.tab.len(), 2);
    assert_eq!(config.prs.tab[0].name, "mine");
    assert_eq!(config.prs.tab[1].name, "review");
    assert_eq!(config.prs.limit, Config::default().prs.limit);
}

#[test]
fn the_demo_keeps_the_users_ui_settings() {
    let user = Config {
        ui: Ui {
            min_column_width: 33,
            ..Default::default()
        },
        ..Default::default()
    };

    let config = demo::config(user);

    assert_eq!(config.ui.min_column_width, 33);
}

#[test]
fn the_demo_queue_of_acme_webapp_holds_two_pull_requests_with_their_runs() {
    let config = demo::config(Config::default());

    let queue = fetch_queue(&Demo, &config.repo[0]).unwrap();

    assert_eq!(queue.entries.len(), 2);
    assert_eq!(queue.entries[0].number, 4821);
    assert_eq!(queue.entries[0].run.as_ref().unwrap().id, 313);
    assert_eq!(queue.entries[1].run.as_ref().unwrap().id, 314);
}

#[test]
fn the_demo_queue_of_acme_infra_is_empty() {
    let config = demo::config(Config::default());

    let queue = fetch_queue(&Demo, &config.repo[1]).unwrap();

    assert!(queue.entries.is_empty());
}

#[test]
fn the_demo_main_builds_are_four_runs_newest_first_with_their_pull_requests() {
    let config = demo::config(Config::default());
    let mut source = BuildsSource::new(config.repo[0].clone());

    let builds = source.fetch(&Demo, SystemTime::now()).unwrap();

    assert_eq!(builds.len(), 4);
    assert_eq!(builds[0].run_number, 312);
    assert_eq!(builds[0].pull.as_ref().unwrap().number, 4840);
    assert_eq!(builds[0].pull.as_ref().unwrap().author, "bob");
    assert_eq!(builds[3].pull.as_ref().unwrap().number, 4790);
}

#[test]
fn a_failed_demo_run_names_its_failed_step() {
    let jobs = fetch_jobs(&Demo, "acme/webapp", 311).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].failed_step.as_deref(), Some("Run cargo test"));
}

#[test]
fn every_demo_run_has_jobs() {
    let jobs = fetch_jobs(&Demo, "acme/webapp", 1).unwrap();

    assert_eq!(jobs.len(), 2);
}
