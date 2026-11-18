use conveyor::config::Config;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{repo, system, to_deployed, two_systems};
use crate::support::{Harness, deployed_in, deployment, key};

#[test]
fn the_title_names_the_shown_tab_before_its_first_fetch() {
    let mut h = Harness::with_config(160, 24, two_systems());
    h.run(to_deployed()).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Deployed worker"), "{screen}");
}

#[test]
fn the_tab_bar_lists_every_system_in_the_narrow_layout_too() {
    let mut h = Harness::with_config(60, 24, two_systems());

    h.run(to_deployed()).unwrap();

    let screen = h.screen();
    assert!(screen.contains("api │ worker"), "{screen}");
}

#[test]
fn the_details_pane_follows_the_shown_tabs_selected_row() {
    let mut h = Harness::with_config(160, 30, two_systems());
    h.run(vec![
        deployed_in(0, "api", vec![deployment("staging", 24, None)]),
        deployed_in(1, "worker", vec![deployment("canary", 31, None)]),
    ])
    .unwrap();
    h.run(to_deployed()).unwrap();
    h.run(vec![key(KeyCode::Char('d'))]).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("canary (worker)"), "{screen}");
}

#[test]
fn a_tenth_deploy_is_reached_with_the_next_tab_key() {
    let mut h = Harness::with_config(160, 24, ten_systems());
    h.run(to_deployed()).unwrap();

    h.run(vec![key(KeyCode::Char('9')), key(KeyCode::Tab)])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Deployed s10"), "{screen}");
}

fn ten_systems() -> Config {
    let mut webapp = repo("acme/webapp");
    webapp.deploy = (1..=10).map(|n| system(&format!("s{n}"))).collect();
    Config {
        repo: vec![webapp],
        ..Config::default()
    }
}
