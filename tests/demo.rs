use conveyor::config::{Config, Prs, Repo, Ui};
use conveyor::sources::demo;

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
