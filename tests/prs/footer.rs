use crate::helpers::three;
use crate::support::{self, Harness, NOW, prs, tick_at};

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
fn a_static_dim_logo_with_the_version_sits_at_the_bottom_right() {
    use ratatui::style::Modifier;
    use support::tick_at_ms;
    let mut h = Harness::new();

    h.run(vec![prs(three()), tick_at_ms(NOW * 1000)]).unwrap();
    let footer = h.screen().lines().last().unwrap().to_string();
    let expected = format!("conveyor v{}", env!("CARGO_PKG_VERSION"));
    assert!(footer.ends_with(&expected), "{footer:?}");
    assert!(
        footer.contains("\u{25aa}\u{2500}\u{25aa}\u{2500}\u{25aa}\u{25b8}"),
        "rollers on a rail: {footer:?}"
    );
    assert!(footer.starts_with("refreshed just now"), "{footer:?}");
    assert!(h.cell(159, 11).modifier.contains(Modifier::DIM));

    h.run(vec![tick_at_ms(NOW * 1000 + 250)]).unwrap();
    assert_eq!(
        h.screen().lines().last().unwrap().to_string(),
        footer,
        "the footer does not move between ticks"
    );
}

const BRAILLE: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn spinner_in(text: &str) -> Option<char> {
    text.chars().find(|c| BRAILLE.contains(c))
}
