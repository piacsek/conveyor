use conveyor::changelog::{CHANGELOG, check};

#[test]
fn the_changelog_has_a_real_entry_for_the_crate_version() {
    assert_eq!(check(CHANGELOG, env!("CARGO_PKG_VERSION")), Ok(()));
}

#[test]
fn an_entry_without_bullets_is_refused() {
    let text = "# Changelog\n\n## v0.2.0 (2026-11-02)\n\n## v0.1.0 (2026-11-01)\n- first\n";

    assert_eq!(
        check(text, "0.2.0"),
        Err("v0.2.0 has no `- ` bullet".to_string())
    );
}

mod support;

use ratatui::crossterm::event::KeyCode;
use support::{Harness, ctrl, key, prs};

#[test]
fn c_opens_the_changelog_at_the_newest_release() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![prs(vec![]), key(KeyCode::Char('c'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Changelog"), "{screen}");
    assert!(
        screen.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))),
        "{screen}"
    );
    assert!(!screen.contains("no open pull requests"), "{screen}");
}

#[test]
fn ctrl_d_scrolls_the_changelog_and_ctrl_u_scrolls_it_back() {
    let mut h = Harness::with_size(80, 12);
    let newest = format!("v{} ", env!("CARGO_PKG_VERSION"));

    h.run(vec![prs(vec![]), key(KeyCode::Char('c')), ctrl('d')])
        .unwrap();
    assert!(h.screen().contains("Changelog"), "{}", h.screen());
    assert!(!h.screen().contains(&newest), "{}", h.screen());

    h.run(vec![ctrl('u')]).unwrap();
    assert!(h.screen().contains(&newest), "{}", h.screen());
}

#[test]
fn a_version_without_an_entry_is_refused() {
    let text = "# Changelog\n\n## v0.1.0 (2026-11-01)\n- first\n";

    assert_eq!(
        check(text, "0.2.0"),
        Err("CHANGELOG.md has no entry for v0.2.0 at the top (found v0.1.0)".to_string())
    );
}

#[test]
fn a_bullet_copied_from_another_release_is_refused() {
    let text =
        "# Changelog\n\n## v0.2.0 (2026-11-02)\n- first\n\n## v0.1.0 (2026-11-01)\n- first\n";

    assert_eq!(
        check(text, "0.2.0"),
        Err("v0.2.0 repeats a bullet of v0.1.0: first".to_string())
    );
}

#[test]
fn versions_out_of_order_or_repeated_are_refused() {
    let text = "# Changelog\n\n## v0.10.0 (2026-11-03)\n- c\n\n## v0.9.0 (2026-11-02)\n- b\n\n## v0.9.0 (2026-11-01)\n- a\n";

    assert_eq!(
        check(text, "0.10.0"),
        Err("v0.9.0 is listed above v0.9.0; newest first, once each".to_string())
    );
}

#[test]
fn a_heading_without_a_date_is_refused() {
    let text = "# Changelog\n\n## v0.2.0\n- second\n\n## v0.1.0 (2026-11-01)\n- first\n";

    assert_eq!(
        check(text, "0.2.0"),
        Err("`## v0.2.0` must read `## vX.Y.Z (YYYY-MM-DD)`".to_string())
    );
}

#[test]
fn j_and_k_scroll_the_changelog_one_line() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('c')),
        key(KeyCode::Char('j')),
    ])
    .unwrap();
    assert!(!h.screen().contains("│Changelog"), "{}", h.screen());
    assert!(h.screen().contains("Changelog ↑↓"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('k'))]).unwrap();
    assert!(h.screen().contains("│Changelog"), "{}", h.screen());
}

#[test]
fn the_footer_says_how_to_scroll_and_leave() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![prs(vec![]), key(KeyCode::Char('c'))]).unwrap();

    let last = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(
        last.starts_with("j/k C-d/C-u scroll · any other key returns"),
        "{last}"
    );
}

#[test]
fn headings_are_bold_without_their_markdown() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![prs(vec![]), key(KeyCode::Char('c'))]).unwrap();

    let screen = h.screen();
    assert!(!screen.contains('#'), "{screen}");
    let row = screen
        .lines()
        .position(|line| line.contains(&format!("│v{} (", env!("CARGO_PKG_VERSION"))))
        .unwrap() as u16;
    assert!(
        h.cell(1, row)
            .modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
}

#[test]
fn the_help_names_the_changelog_key() {
    let mut h = Harness::with_size(80, 17);

    h.run(vec![prs(vec![]), key(KeyCode::Char('?'))]).unwrap();

    assert!(h.screen().contains("?/c"), "{}", h.screen());
    assert!(h.screen().contains("help / changelog"), "{}", h.screen());
}

#[test]
fn a_bullet_without_text_is_refused() {
    let text = "# Changelog\n\n## v0.2.0 (2026-11-02)\n- \n\n## v0.1.0 (2026-11-01)\n- first\n";

    assert_eq!(
        check(text, "0.2.0"),
        Err("v0.2.0 has no `- ` bullet".to_string())
    );
}

#[test]
fn a_bullet_continued_on_the_next_line_is_one_bullet() {
    let text = "# Changelog\n\n## v0.2.0 (2026-11-02)\n- Fixed\n  the queue refresh\n\n## v0.1.0 (2026-11-01)\n- Fixed\n  the build refresh\n";

    assert_eq!(check(text, "0.2.0"), Ok(()));
}

#[test]
fn long_bullets_wrap_inside_a_narrow_pane() {
    let mut h = Harness::with_size(40, 12);

    h.run(vec![prs(vec![]), key(KeyCode::Char('c'))]).unwrap();

    assert!(h.screen().contains("key returns."), "{}", h.screen());
}

#[test]
fn q_closes_the_changelog_instead_of_quitting() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('c')),
        key(KeyCode::Char('q')),
        prs(vec![]),
    ])
    .unwrap();

    assert!(
        h.screen().contains("no open pull requests"),
        "{}",
        h.screen()
    );
}

#[test]
fn c_is_ordinary_text_in_a_filter() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('c')),
    ])
    .unwrap();

    assert!(!h.screen().contains("Changelog"), "{}", h.screen());
    assert!(h.screen().contains("/c▏"), "{}", h.screen());
}
