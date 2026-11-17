use ratatui::crossterm::event::KeyCode;

use crate::helpers::{open, tall};
use crate::support::{key, prs};

#[test]
fn a_title_of_wide_characters_ends_inside_the_confirm_dialog() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "acme: 修复 🚀 deploy the webhook retries")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("retries? │"), "{screen}");
}
