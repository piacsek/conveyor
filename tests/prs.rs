mod support;

use ratatui::crossterm::event::KeyCode;
use support::{Harness, NOW, key, pr, prs, tick_at};

#[test]
fn no_pull_requests_shows_an_empty_column_and_q_quits() {
    let mut h = Harness::new();

    h.run(vec![prs(vec![]), key(KeyCode::Char('q'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs (0)"), "{screen}");
    assert!(screen.contains("no open pull requests"), "{screen}");
    assert!(h.opener.opened().is_empty());
}

#[test]
fn the_column_says_fetching_until_the_first_data_arrives() {
    let mut h = Harness::new();

    h.run(vec![key(KeyCode::Char('q'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("fetching…"), "{screen}");
    assert!(!screen.contains("no open pull requests"), "{screen}");
}

#[test]
fn cards_stack_a_title_a_dim_meta_line_and_the_unhappy_checks_under_a_selection_bar() {
    let mut h = Harness::new();
    let mut first = pr(4821, "Retry hooks");
    first.head_ref = "webhook-retry".to_string();
    first.additions = 12;
    first.deletions = 3;

    h.run(vec![
        prs(vec![first, pr(4830, "Rate limits")]),
        key(KeyCode::Char('q')),
    ])
    .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[0].contains("My PRs (2)"), "{screen}");
    assert!(rows[1].starts_with("│▌ ✓ #4821 Retry hooks"), "{screen}");
    assert!(rows[1].contains("2h│"), "{screen}");
    assert!(
        rows[2].starts_with("│    webapp · webhook-retry · +12 −3"),
        "{screen}"
    );
    assert!(rows[3].starts_with("│    checks: success"), "{screen}");
    assert!(rows[4].starts_with("│  ✓ #4830 Rate limits"), "{screen}");
    assert!(!screen.contains("1 ✓"), "no row numbers: {screen}");
}

fn three() -> Vec<conveyor::model::prs::PullRequest> {
    vec![pr(1, "a"), pr(2, "b"), pr(3, "c")]
}

fn highlighted_row(screen: &str) -> usize {
    screen
        .lines()
        .position(|line| line.starts_with("│▌"))
        .expect("a highlighted row")
}

#[test]
fn j_k_arrows_gg_and_g_move_the_highlight_and_clamp() {
    let mut h = Harness::new();
    h.run(vec![
        prs(three()),
        key(KeyCode::Char('j')),
        key(KeyCode::Down),
    ])
    .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 7);

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('j'))])
        .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 7, "clamps at the end");

    h.run(vec![
        key(KeyCode::Char('k')),
        key(KeyCode::Up),
        key(KeyCode::Up),
    ])
    .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 1, "clamps at the start");

    h.run(vec![key(KeyCode::Char('G'))]).unwrap();
    assert_eq!(highlighted_row(&h.screen()), 7);

    h.run(vec![key(KeyCode::Char('g')), key(KeyCode::Char('g'))])
        .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 1);
}

#[test]
fn enter_and_o_open_the_selected_pull_request_and_keep_running() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('j')),
        key(KeyCode::Enter),
        key(KeyCode::Char('o')),
        key(KeyCode::Char('q')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/pull/2".to_string()],
        "the second press within a second is debounced"
    );
    assert_eq!(
        highlighted_row(&h.screen()),
        4,
        "still open on the same row"
    );
}

#[test]
fn y_copies_the_url_and_the_footer_confirms_it() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(
        h.opener.copied(),
        vec!["https://github.com/acme/webapp/pull/1".to_string()]
    );
    let screen = h.screen();
    assert!(screen.contains("copied #1"), "{screen}");

    h.run(vec![key(KeyCode::Char('j'))]).unwrap();
    assert!(!h.screen().contains("copied #1"), "cleared by the next key");
}

#[test]
fn p_toggles_a_details_pane_with_failed_checks_first() {
    use conveyor::model::prs::{Check, CheckConclusion, MergeState, ReviewDecision};
    let mut h = Harness::with_size(160, 24);
    let mut detailed = pr(4821, "Retry hooks");
    detailed.head_ref = "webhook-retry".to_string();
    detailed.additions = 12;
    detailed.deletions = 3;
    detailed.review = ReviewDecision::Approved;
    detailed.merge_state = MergeState::Blocked;
    detailed.checks_detail = vec![
        Check {
            name: "lint".to_string(),
            conclusion: CheckConclusion::Success,
            url: "https://ci/lint".to_string(),
        },
        Check {
            name: "api / test".to_string(),
            conclusion: CheckConclusion::Failure,
            url: "https://ci/test".to_string(),
        },
    ];

    h.run(vec![prs(vec![detailed]), key(KeyCode::Char('p'))])
        .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("#4821 acme/webapp  webhook-retry  +12 −3"),
        "{screen}"
    );
    assert!(screen.contains("review: approved"), "{screen}");
    assert!(screen.contains("merge: blocked"), "{screen}");
    let test_line = screen
        .lines()
        .position(|l| l.contains("✗ api / test"))
        .unwrap();
    let lint_line = screen.lines().position(|l| l.contains("✓ lint")).unwrap();
    assert!(test_line < lint_line, "failures first: {screen}");

    h.run(vec![key(KeyCode::Char('p'))]).unwrap();
    assert!(!h.screen().contains("review: approved"), "{}", h.screen());
    assert!(!h.screen().contains("✓ lint"), "{}", h.screen());
}

#[test]
fn slash_filters_rows_by_number_title_or_repo_and_shows_the_count() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(1, "alpha"), pr(2, "beta"), pr(3, "gamma")]),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('a')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("#3 gamma"), "{screen}");
    assert!(!screen.contains("alpha"), "{screen}");
    assert!(screen.contains("/am  1/3"), "{screen}");

    h.run(vec![key(KeyCode::Char('z'))]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("no matches for /amz"), "{screen}");

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("alpha") && screen.contains("gamma"),
        "{screen}"
    );
    assert!(!screen.contains("/amz"), "{screen}");
}

#[test]
fn r_requests_a_refresh_and_the_footer_shows_the_data_age() {
    use conveyor::app::Stage;
    let mut h = Harness::new();

    h.run(vec![prs(three()), tick_at(NOW + 12)]).unwrap();
    assert!(
        h.screen().contains("refreshed at 08:00:00"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('r'))]).unwrap();
    assert_eq!(
        h.requests,
        vec![conveyor::app::Request::Refresh(Stage::Prs)]
    );
}

#[test]
fn a_failed_fetch_keeps_the_old_rows_and_flags_the_column_until_the_next_success() {
    use conveyor::app::Stage;
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        support::failed(Stage::Prs, "gh: HTTP 401: Bad credentials"),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs (3) ⚠"), "{screen}");
    assert!(screen.contains("#1 a"), "rows kept: {screen}");
    assert!(screen.contains("gh: HTTP 401: Bad credentials"), "{screen}");

    h.run(vec![prs(three())]).unwrap();
    let screen = h.screen();
    assert!(!screen.contains("⚠"), "{screen}");
    assert!(!screen.contains("Bad credentials"), "{screen}");
}

#[test]
fn a_failed_open_shows_in_the_footer_and_the_app_stays_up() {
    let mut h = Harness::new();
    h.opener.fail_next("open: exec failed");

    h.run(vec![prs(three()), key(KeyCode::Enter)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("open: exec failed"), "{screen}");
    assert!(screen.contains("#1 a"), "{screen}");
}

#[test]
fn a_refresh_keeps_the_row_order_and_the_selection_follows_the_number() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('j'))]).unwrap();
    h.run(vec![prs(vec![pr(4, "d"), pr(3, "c"), pr(2, "b")])])
        .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].contains("#2 b"), "kept: {screen}");
    assert!(rows[4].contains("#3 c"), "kept: {screen}");
    assert!(rows[7].contains("#4 d"), "appended: {screen}");
    assert!(!screen.contains("#1 a"), "gone: {screen}");
    assert_eq!(highlighted_row(&screen), 1, "selection follows #2");

    h.run(vec![prs(vec![pr(4, "d")])]).unwrap();
    assert_eq!(
        highlighted_row(&h.screen()),
        1,
        "falls back to the first row"
    );
}

#[test]
fn question_mark_shows_the_key_help_and_any_key_returns() {
    let mut h = Harness::with_size(160, 16);

    h.run(vec![prs(three()), key(KeyCode::Char('?'))]).unwrap();

    let screen = h.screen();
    for needle in [
        "j/k",
        "Enter/o",
        "open in browser",
        "b",
        "open build",
        "R",
        "refresh all",
        "y",
        "copy URL",
        "p",
        "details",
        "/",
        "filter",
        "r",
        "refresh",
        "h/l",
        "column",
        "q",
        "quit",
    ] {
        assert!(screen.contains(needle), "{needle}: {screen}");
    }
    assert!(!screen.contains("#1 a"), "{screen}");

    h.run(vec![key(KeyCode::Char('x'))]).unwrap();
    assert!(h.screen().contains("#1 a"), "{}", h.screen());
}

#[test]
fn narrow_terminals_collapse_the_columns_into_tabs_and_h_l_switch_them() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![prs(three())]).unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("My PRs (3) │ Merge queue │ Main builds │ Deployed"),
        "{screen}"
    );
    assert!(screen.contains("#1 a"), "{screen}");
    assert!(!screen.contains("fetching…"), "{screen}");

    h.run(vec![key(KeyCode::Char('l'))]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("fetching…"), "the queue column: {screen}");
    assert!(!screen.contains("#1 a"), "{screen}");

    h.run(vec![key(KeyCode::Char('h')), key(KeyCode::Char('h'))])
        .unwrap();
    assert!(h.screen().contains("#1 a"), "clamps at the first column");

    h.run(vec![
        key(KeyCode::Tab),
        key(KeyCode::Tab),
        key(KeyCode::Tab),
        key(KeyCode::Tab),
    ])
    .unwrap();
    assert!(h.screen().contains("#1 a"), "Tab wraps around");
}

#[test]
fn a_failure_before_any_data_flags_the_column_and_shows_the_message_in_the_body() {
    use conveyor::app::Stage;
    let mut h = Harness::new();

    h.run(vec![support::failed(
        Stage::Prs,
        "gh: HTTP 401: Bad credentials",
    )])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs ⚠"), "{screen}");
    assert!(
        !screen.lines().nth(1).unwrap().starts_with("│fetching…"),
        "{screen}"
    );
    assert!(
        screen
            .lines()
            .nth(1)
            .unwrap()
            .contains("gh: HTTP 401: Bad credentials"),
        "{screen}"
    );
}

#[test]
fn check_glyphs_are_colored_by_state_and_the_age_is_dim() {
    use conveyor::model::prs::CheckState;
    use ratatui::style::{Color, Modifier};
    let mut h = Harness::new();
    let mut failing = pr(1, "a");
    failing.checks = CheckState::Failure;
    let mut pending = pr(2, "b");
    pending.checks = CheckState::Pending;
    let ok = pr(3, "c");

    h.run(vec![prs(vec![failing, pending, ok])]).unwrap();

    assert_eq!(h.cell(3, 1).fg, Color::Red);
    assert_eq!(h.cell(3, 4).fg, Color::Yellow);
    assert_eq!(h.cell(3, 7).fg, Color::Green);
    assert!(h.cell(37, 1).modifier.contains(Modifier::DIM), "age is dim");
    assert!(h.cell(1, 1).fg == Color::Cyan, "the selection bar is cyan");
    assert!(
        h.cell(6, 2).modifier.contains(Modifier::DIM),
        "the meta line is dim"
    );
}

#[test]
fn the_tab_layout_does_not_repeat_the_column_title() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![prs(three())]).unwrap();

    let screen = h.screen();
    assert_eq!(screen.matches("My PRs (3)").count(), 1, "{screen}");
}

#[test]
fn a_pull_request_in_the_merge_queue_shows_its_position_on_the_row() {
    let mut h = Harness::new();
    let mut queued = pr(4821, "Retry hooks");
    queued.queue_position = Some(2);

    h.run(vec![prs(vec![queued, pr(4830, "Rate limits")])])
        .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].contains("#4821 Retry hooks"), "{screen}");
    assert!(rows[1].contains("⇥2 2h"), "{screen}");
    assert!(!rows[4].contains("⇥"), "{screen}");
}

#[test]
fn a_held_enter_opens_the_same_row_only_once_per_second() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        key(KeyCode::Enter),
        key(KeyCode::Enter),
        key(KeyCode::Char('o')),
        tick_at(NOW + 2),
        key(KeyCode::Enter),
        key(KeyCode::Char('j')),
        key(KeyCode::Enter),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec![
            "https://github.com/acme/webapp/pull/1".to_string(),
            "https://github.com/acme/webapp/pull/1".to_string(),
            "https://github.com/acme/webapp/pull/2".to_string(),
        ]
    );
}

#[test]
fn the_footer_says_just_now_for_three_seconds_then_the_wall_clock_time() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), tick_at(NOW + 2)]).unwrap();
    assert!(h.screen().contains("refreshed just now"), "{}", h.screen());

    h.run(vec![tick_at(NOW + 12)]).unwrap();
    assert!(
        h.screen().contains("refreshed at 08:00:00"),
        "{}",
        h.screen()
    );
    assert!(!h.screen().contains("ago"), "{}", h.screen());

    h.app.utc_offset_secs = -3 * 3600;
    h.run(vec![tick_at(NOW + 13)]).unwrap();
    assert!(
        h.screen().contains("refreshed at 05:00:00"),
        "{}",
        h.screen()
    );
}

const BRAILLE: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn spinner_in(text: &str) -> Option<char> {
    text.chars().find(|c| BRAILLE.contains(c))
}

#[test]
fn a_refreshing_column_spins_one_braille_glyph_at_the_start_of_the_footer() {
    use conveyor::app::Stage;
    use support::{fetching, tick_at_ms};
    let mut h = Harness::new();

    h.run(vec![prs(three()), fetching(Stage::Prs)]).unwrap();
    let footer = h.screen().lines().last().unwrap().to_string();
    let first = spinner_in(&footer).expect("a spinner in the footer");
    assert!(footer.starts_with(first), "at the very start: {footer:?}");
    assert!(footer.contains("refreshed"), "{footer:?}");
    assert_eq!(
        spinner_in(h.screen().lines().next().unwrap()),
        None,
        "no spinner in the column title"
    );

    h.run(vec![tick_at_ms(NOW * 1000 + 100)]).unwrap();
    let second = spinner_in(h.screen().lines().last().unwrap()).unwrap();
    assert_ne!(first, second, "the spinner advances with time");

    h.run(vec![prs(three())]).unwrap();
    assert_eq!(spinner_in(h.screen().lines().last().unwrap()), None);
}

#[test]
fn the_initial_fetch_spins_too_and_the_loading_body_stays_plain() {
    use conveyor::app::Stage;
    use support::fetching;
    let mut h = Harness::new();

    h.run(vec![fetching(Stage::Prs)]).unwrap();

    let body = h.screen().lines().nth(1).unwrap().to_string();
    assert!(body.contains("fetching…"), "{body}");
    assert_eq!(spinner_in(&body), None, "{body}");
    assert!(
        spinner_in(h.screen().lines().last().unwrap()).is_some(),
        "{}",
        h.screen()
    );
}

#[test]
fn any_refreshing_column_spins_the_footer_even_when_another_one_is_focused() {
    use conveyor::app::Stage;
    use support::{fetching, queue};
    let mut h = Harness::new();

    h.run(vec![prs(three()), fetching(Stage::Queue)]).unwrap();
    assert!(
        spinner_in(h.screen().lines().last().unwrap()).is_some(),
        "{}",
        h.screen()
    );

    h.run(vec![queue(vec![])]).unwrap();
    assert_eq!(spinner_in(h.screen().lines().last().unwrap()), None);
}

#[test]
fn shift_r_refreshes_every_column() {
    use conveyor::app::{Request, Stage};
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('R'))]).unwrap();

    assert_eq!(
        h.requests,
        Stage::ALL.map(Request::Refresh).to_vec(),
        "one request per column"
    );
}

#[test]
fn a_static_dim_logo_with_the_version_sits_at_the_bottom_right() {
    use ratatui::style::Modifier;
    use support::tick_at_ms;
    let mut h = Harness::new();

    h.run(vec![prs(three()), tick_at_ms(NOW * 1000)]).unwrap();
    let footer = h.screen().lines().last().unwrap().to_string();
    let expected = format!("conveyor v{}", env!("CARGO_PKG_VERSION"));
    assert!(footer.ends_with(&expected), "{footer:?}");
    assert!(footer.contains('\u{25a3}'), "a conveyor logo: {footer:?}");
    assert!(footer.starts_with("refreshed just now"), "{footer:?}");
    assert!(h.cell(159, 11).modifier.contains(Modifier::DIM));

    h.run(vec![tick_at_ms(NOW * 1000 + 250)]).unwrap();
    assert_eq!(
        h.screen().lines().last().unwrap().to_string(),
        footer,
        "the footer does not move between ticks"
    );
}

#[test]
fn b_opens_the_check_run_behind_a_pull_request_and_says_so_when_there_is_none() {
    use conveyor::model::prs::{Check, CheckConclusion};
    let mut h = Harness::new();
    let mut checked = pr(4821, "Retry hooks");
    checked.checks_detail = vec![
        Check {
            name: "lint".to_string(),
            conclusion: CheckConclusion::Success,
            url: "https://ci/lint".to_string(),
        },
        Check {
            name: "api / test".to_string(),
            conclusion: CheckConclusion::Failure,
            url: "https://ci/test".to_string(),
        },
    ];

    h.run(vec![
        prs(vec![checked, pr(4830, "Rate limits")]),
        key(KeyCode::Char('b')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://ci/test".to_string()],
        "the failing check wins"
    );

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('b'))])
        .unwrap();
    assert_eq!(h.opener.opened().len(), 1, "no second open");
    assert!(h.screen().contains("no checks yet"), "{}", h.screen());
}
