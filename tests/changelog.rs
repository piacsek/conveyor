use conveyor::changelog::{CHANGELOG, check, check_released};

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

use conveyor::app::Input;
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
    assert!(!screen.contains("│#"), "{screen}");
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
    let mut h = Harness::with_size(24, 12);
    h.app.changelog =
        "# Changelog\n\n## v0.1.0 (2026-11-01)\n- one two three four five six seven\n";

    h.run(vec![prs(vec![]), key(KeyCode::Char('c'))]).unwrap();

    assert!(
        h.screen().contains("│- one two three four "),
        "{}",
        h.screen()
    );
    assert!(h.screen().contains("│  five six seven "), "{}", h.screen());
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

#[test]
fn a_second_level_heading_that_is_not_a_release_is_refused() {
    let text = "# Changelog\n\n## v0.21.0 (2026-11-02)\n\n## 0.20.0 (2026-11-02)\n- old\n";

    assert_eq!(
        check(text, "0.21.0"),
        Err("`## 0.20.0 (2026-11-02)` must read `## vX.Y.Z (YYYY-MM-DD)`".to_string())
    );
}

fn rendered(markdown: &str, width: usize) -> Vec<String> {
    conveyor::ui::changelog::lines(markdown, width)
        .iter()
        .map(|line| line.to_string())
        .collect()
}

#[test]
fn a_long_bullet_hangs_under_its_dash() {
    let lines = rendered("- one two three four", 9);

    assert_eq!(lines.first().map(String::as_str), Some("- one two"));
    assert_eq!(lines.get(1).map(String::as_str), Some("  three"));
    assert_eq!(lines.get(2).map(String::as_str), Some("  four"));
}

#[test]
fn a_continuation_line_hangs_like_its_bullet() {
    let lines = rendered("  three four", 9);

    assert_eq!(lines.first().map(String::as_str), Some("  three"));
    assert_eq!(lines.get(1).map(String::as_str), Some("  four"));
}

#[test]
fn a_released_version_without_its_entry_is_refused() {
    let text = "# Changelog\n\n## v0.21.0 (2026-11-02)\n- relabelled\n";

    assert_eq!(
        check_released(text, &["0.20.0", "0.21.0"], "0.21.0"),
        Err("v0.20.0 was released but has no entry; restore it, or merge origin/main if another branch released it".to_string())
    );
}

#[test]
fn every_released_tag_keeps_its_entry() {
    let output = std::process::Command::new("git")
        .args(["tag", "--list", "v*"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    let tags = String::from_utf8(output.stdout).unwrap();
    let released: Vec<&str> = tags
        .lines()
        .filter_map(|tag| tag.strip_prefix('v'))
        .collect();

    assert!(
        !released.is_empty(),
        "no v* tags: run `git fetch --tags` (CI needs fetch-depth: 0)"
    );
    assert_eq!(
        check_released(CHANGELOG, &released, env!("CARGO_PKG_VERSION")),
        Ok(())
    );
}

#[test]
fn arrows_scroll_the_changelog_like_j_and_k() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('c')),
        key(KeyCode::Down),
    ])
    .unwrap();
    assert!(!h.screen().contains("│Changelog"), "{}", h.screen());

    h.run(vec![key(KeyCode::Up)]).unwrap();
    assert!(h.screen().contains("│Changelog"), "{}", h.screen());
}

#[test]
fn ctrl_c_quits_from_the_changelog() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('c')),
        ctrl('c'),
        key(KeyCode::Esc),
    ])
    .unwrap();

    assert!(h.screen().contains("│Changelog"), "{}", h.screen());
}

#[test]
fn deeper_headings_lose_their_hashes_too() {
    assert_eq!(rendered("### Added", 20), vec!["Added".to_string()]);
}

#[test]
fn reopening_the_changelog_starts_at_the_top() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('c')),
        ctrl('d'),
        key(KeyCode::Esc),
        key(KeyCode::Char('c')),
    ])
    .unwrap();

    assert!(h.screen().contains("│Changelog"), "{}", h.screen());
}

#[test]
fn a_tag_newer_than_the_crate_version_is_not_required() {
    let text = "# Changelog\n\n## v0.20.0 (2026-11-02)\n- now\n";

    assert_eq!(
        check_released(text, &["0.20.0", "0.21.0"], "0.20.0"),
        Ok(())
    );
}

#[test]
fn a_result_that_lands_while_the_changelog_is_open_shows_in_its_footer() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('c')),
        Ok(Input::Merged(7, Err("merge refused".to_string()))),
    ])
    .unwrap();

    let last = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(last.starts_with("#7: merge refused"), "{last}");
}

#[test]
fn a_result_that_lands_while_the_help_is_open_shows_in_its_footer() {
    let mut h = Harness::with_size(80, 17);

    h.run(vec![
        prs(vec![]),
        key(KeyCode::Char('?')),
        Ok(Input::Merged(7, Err("merge refused".to_string()))),
    ])
    .unwrap();

    let last = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(last.starts_with("#7: merge refused"), "{last}");
}

#[test]
fn a_date_that_is_not_on_the_calendar_is_refused() {
    let text =
        "# Changelog\n\n## v0.2.0 (2026-02-30)\n- second\n\n## v0.1.0 (2026-11-01)\n- first\n";

    assert_eq!(
        check(text, "0.2.0"),
        Err("`## v0.2.0 (2026-02-30)` must read `## vX.Y.Z (YYYY-MM-DD)`".to_string())
    );
}

#[test]
fn a_leap_day_is_a_date() {
    let text =
        "# Changelog\n\n## v0.2.0 (2028-02-29)\n- second\n\n## v0.1.0 (2026-11-01)\n- first\n";

    assert_eq!(check(text, "0.2.0"), Ok(()));
}
