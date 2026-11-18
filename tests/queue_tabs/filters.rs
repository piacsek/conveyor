use conveyor::config::{Config, PrsTab};
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::repo;
use crate::support::{Harness, build, builds_in, entry, key, pr, prs_in, queue_in};

#[test]
fn a_my_prs_switch_keeps_the_build_main_builds_selected_under_the_filter() {
    let mut h = Harness::with_config(160, 24, two_repos_and_two_prs_tabs());
    h.run(vec![
        builds_in(
            0,
            "acme/monorepo",
            vec![
                build(
                    7,
                    BuildStatus::Success,
                    Some((12, "alice", "Retry webhooks")),
                ),
                build(8, BuildStatus::Success, Some((13, "carol", "Rate limits"))),
            ],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ])
    .unwrap();
    h.run(typed_filter("Retry")).unwrap();

    h.run(vec![
        key(KeyCode::Char('h')),
        key(KeyCode::Char('h')),
        key(KeyCode::Tab),
    ])
    .unwrap();

    assert_eq!(
        h.app.builds.active().column.selected().unwrap().run_number,
        7
    );
}

#[test]
fn a_my_prs_switch_keeps_the_entry_the_queue_selected_under_the_filter() {
    let mut h = Harness::with_config(160, 24, two_repos_and_two_prs_tabs());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![
                entry(1, 12, "alice", "Retry webhooks"),
                entry(2, 13, "carol", "Rate limits"),
            ],
        ),
        key(KeyCode::Char('l')),
    ])
    .unwrap();
    h.run(typed_filter("Rate")).unwrap();

    h.run(vec![key(KeyCode::Char('h')), key(KeyCode::Tab)])
        .unwrap();

    assert_eq!(h.app.queue.column().selected().unwrap().number, 13);
}

#[test]
fn a_repository_switch_keeps_the_pull_request_my_prs_selected_under_the_filter() {
    let mut h = Harness::with_config(160, 24, two_repos_and_two_prs_tabs());
    h.run(vec![
        prs_in(0, vec![pr(12, "Retry webhooks"), pr(13, "Rate limits")]),
        key(KeyCode::Char('l')),
    ])
    .unwrap();
    h.run(typed_filter("Rate")).unwrap();
    assert_eq!(h.app.prs.column().selected().unwrap().number, 13);

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.prs.column().selected().unwrap().number, 13);
}

fn typed_filter(query: &str) -> Vec<std::io::Result<conveyor::app::Input>> {
    let mut keys = vec![key(KeyCode::Char('/'))];
    keys.extend(query.chars().map(|c| key(KeyCode::Char(c))));
    keys.push(key(KeyCode::Enter));
    keys
}

fn two_repos_and_two_prs_tabs() -> Config {
    let mut config = Config {
        repo: vec![repo("acme/monorepo"), repo("acme/infra")],
        ..Config::default()
    };
    config.prs.tab = vec![prs_tab("mine"), prs_tab("review")];
    config
}

fn prs_tab(name: &str) -> PrsTab {
    PrsTab {
        name: name.to_string(),
        query: format!("is:open {name}"),
        limit: None,
    }
}
