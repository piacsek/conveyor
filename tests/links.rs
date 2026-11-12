mod support;

use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, build, builds, deployed, deployment, entry, key, pr, prs, queue};

fn link(number: u64) -> (String, String) {
    (
        format!("#{number}"),
        format!("https://github.com/acme/webapp/pull/{number}"),
    )
}

#[test]
fn every_pull_request_number_in_my_prs_links_to_its_pull_request() {
    let mut h = Harness::new();

    h.run(vec![prs(vec![pr(1, "a"), pr(22, "b"), pr(333, "c")])])
        .unwrap();

    assert_eq!(h.links(), vec![link(1), link(22), link(333)]);
}

#[test]
fn the_card_reads_and_looks_the_same_with_its_number_linked() {
    use ratatui::style::Modifier;
    let mut h = Harness::new();

    h.run(vec![prs(vec![pr(4821, "Retry hooks"), pr(4830, "Limits")])])
        .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(
        rows[1].starts_with("│▌ ✓ \u{f407} #4821 Retry hooks"),
        "{screen}"
    );
    assert!(
        h.cell(7, 1).modifier.contains(Modifier::BOLD),
        "the selected number stays bold"
    );
    assert!(
        !h.cell(7, 4).modifier.contains(Modifier::BOLD),
        "the other number stays plain"
    );
}

#[test]
fn the_numbers_in_the_queue_main_builds_and_deployed_link_too() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(1, "a")]),
        queue(vec![entry(1, 4821, "alice", "Retry webhooks")]),
        builds(vec![build(
            25,
            BuildStatus::Success,
            Some((4822, "bob", "Speed up CI")),
        )]),
        deployed(vec![deployment(
            "prod",
            7,
            Some((4823, "carol", "Rate limits")),
        )]),
    ])
    .unwrap();

    assert_eq!(h.links(), vec![link(1), link(4821), link(4822), link(4823)]);
}

#[test]
fn a_build_that_only_knows_its_number_links_to_the_pull_request_p_opens() {
    let mut h = Harness::new();
    let mut squashed = build(25, BuildStatus::Success, Some((4822, "bob", "Speed up CI")));
    squashed.pull = None;

    h.run(vec![
        builds(vec![squashed, build(24, BuildStatus::Success, None)]),
        key(KeyCode::Char('3')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert_eq!(h.links(), vec![link(4822)], "run 24 has no pull request");
    assert_eq!(h.opener.opened(), vec![link(4822).1]);
}

#[test]
fn the_links_follow_the_cards_when_the_column_scrolls() {
    let mut h = Harness::new();
    let many = (1..=6).map(|n| pr(n, "t")).collect();

    h.run(vec![
        prs(many),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('j')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        !screen.contains("#1 t"),
        "the first card scrolled out: {screen}"
    );
    let on_screen: Vec<(String, String)> = (1..=6)
        .filter(|number| screen.contains(&format!("#{number} t")))
        .map(link)
        .collect();
    assert!(on_screen.contains(&link(5)), "{screen}");
    assert_eq!(h.links(), on_screen, "{screen}");
}

#[test]
fn a_url_that_is_not_a_plain_web_url_is_never_written_as_a_link() {
    let mut h = Harness::new();
    let mut local = pr(1, "a");
    local.url = "file:///etc/passwd".to_string();
    let mut escape = pr(2, "b");
    escape.url = "https://github.com/acme/webapp/pull/2\x1b]52;c;eA==\x07".to_string();
    let mut spaced = pr(3, "c");
    spaced.url = "https://github.com/acme/webapp/pull/3 x".to_string();

    h.run(vec![prs(vec![local, escape, spaced])]).unwrap();

    assert!(h.links().is_empty(), "{:?}", h.links());
    let screen = h.screen();
    assert!(screen.contains("#1 a"), "{screen}");
    assert!(!screen.contains('\x1b'), "no escape reaches the screen");
}

#[test]
fn a_popup_drawn_over_a_number_takes_its_link_away() {
    let mut h = Harness::new();

    h.run(vec![prs(vec![pr(1, "a")])]).unwrap();
    assert_eq!(h.links(), vec![link(1)]);

    h.run(vec![key(KeyCode::Char('?'))]).unwrap();

    assert!(h.links().is_empty(), "{:?}", h.links());
    assert!(!h.screen().contains('\x1b'), "no escape is left behind");
}
