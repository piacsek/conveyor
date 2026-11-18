use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::two_repos;
use crate::support::{Harness, build, builds_in, ctrl, entry, job, jobs, key, queue_in};

#[test]
fn a_round_trip_from_main_builds_keeps_its_selection_and_drops_the_filter() {
    let mut h = Harness::with_config(160, 24, two_repos());
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
        key(KeyCode::Char('j')),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('R')),
        key(KeyCode::Char('a')),
        key(KeyCode::Char('t')),
        key(KeyCode::Enter),
        key(KeyCode::Tab),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    assert_eq!(h.app.query, None);
    assert_eq!(
        h.app.builds.active().column.selected().unwrap().run_number,
        8
    );
    assert!(h.screen().contains("Retry webhooks"), "{}", h.screen());
}

#[test]
fn a_round_trip_from_main_builds_keeps_the_queue_selection() {
    let mut h = Harness::with_config(160, 24, two_repos());
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
        key(KeyCode::Char('j')),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    assert_eq!(h.app.queue.column().selected().unwrap().number, 13);
}

#[test]
fn a_switch_from_main_builds_puts_the_details_back_at_the_top() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        builds_in(
            0,
            "acme/monorepo",
            vec![build(7, BuildStatus::Success, None)],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
        jobs(1007, many_jobs(24)),
        ctrl('d'),
    ])
    .unwrap();
    assert_ne!(h.app.details_scroll, 0);

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.details_scroll, 0);
    assert_eq!(h.app.log_scroll, u16::MAX);
}

fn many_jobs(count: u64) -> Vec<conveyor::model::jobs::Job> {
    (1..=count)
        .map(|i| job(i, &format!("job {i}"), BuildStatus::Success, None))
        .collect()
}
