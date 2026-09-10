mod support;

use ratatui::crossterm::event::KeyCode;
use support::{Harness, key, prs};

#[test]
fn no_pull_requests_shows_an_empty_column_and_q_quits() {
    let mut h = Harness::new();

    h.run(vec![prs(vec![]), key(KeyCode::Char('q'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs (0)"), "{screen}");
    assert!(screen.contains("no open pull requests"), "{screen}");
    assert!(h.opener.opened().is_empty());
}
