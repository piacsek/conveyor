mod support;

use conveyor::app::Stage;
use conveyor::config::Config;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, build, builds, deployed, deployment, entry, failed, key, pr, prs, queue};

#[test]
fn a_custom_title_names_the_column_before_its_count_and_warning() {
    let mut h = Harness::with_config(160, 12, titled());

    h.run(vec![
        prs(vec![pr(12, "Retry webhooks")]),
        failed(Stage::Prs, "HTTP 502"),
    ])
    .unwrap();

    assert!(h.screen().contains("┌Mine (1) ⚠─"), "{}", h.screen());
}

#[test]
fn a_custom_title_stands_alone_before_the_repository_is_known() {
    let mut h = Harness::with_config(160, 12, titled());

    h.run(vec![]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("┌Line─"), "{screen}");
    assert!(screen.contains("┌Trunk─"), "{screen}");
    assert!(screen.contains("┌Envs─"), "{screen}");
}

#[test]
fn a_custom_title_keeps_the_repository_or_system_after_it() {
    let mut h = Harness::with_config(160, 12, titled());

    h.run(vec![
        queue(vec![entry(1, 4821, "alice", "Retry webhooks")]),
        builds(vec![build(25, BuildStatus::Success, None)]),
        deployed(vec![deployment("staging", 1, None)]),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("┌Line webapp (1)─"), "{screen}");
    assert!(screen.contains("┌Trunk webapp (1)─"), "{screen}");
    assert!(screen.contains("┌Envs api (1)─"), "{screen}");
}

#[test]
fn the_column_bar_of_a_narrow_terminal_shows_the_custom_titles() {
    let mut h = Harness::with_config(80, 12, titled());

    h.run(vec![prs(vec![pr(12, "Retry webhooks")])]).unwrap();

    let bar = h.screen().lines().next().unwrap_or_default().to_string();
    assert!(bar.contains("Mine (1)"), "{bar}");
    assert!(bar.contains("Line"), "{bar}");
    assert!(bar.contains("Trunk"), "{bar}");
    assert!(bar.contains("Envs"), "{bar}");
}

#[test]
fn the_help_header_names_the_column_by_its_custom_title() {
    let mut h = Harness::with_config(160, 20, titled());

    h.run(vec![key(KeyCode::Char('l')), key(KeyCode::Char('?'))])
        .unwrap();

    assert!(h.screen().contains("Keys — Line"), "{}", h.screen());
}

fn titled() -> Config {
    let mut config = Config::default();
    config.ui.titles.prs = Some("Mine".to_string());
    config.ui.titles.queue = Some("Line".to_string());
    config.ui.titles.builds = Some("Trunk".to_string());
    config.ui.titles.deployed = Some("Envs".to_string());
    config
}
