use std::time::Duration;

use conveyor::text::{cells, elapsed, scroll, truncate, wrap};

const WIDE: &str = "acme: 修复 🚀 deploy";

#[test]
fn truncate_counts_a_wide_character_as_two_cells() {
    assert_eq!(truncate(WIDE, 20), WIDE);
    assert_eq!(truncate(WIDE, 18), "acme: 修复 🚀 dep…");
    assert_eq!(truncate(WIDE, 10), "acme: 修…");
}

#[test]
fn wrap_fills_words_greedily_up_to_the_width() {
    assert_eq!(wrap("one two three", 9, 3), vec!["one two", "three"]);
    assert_eq!(wrap("one two three", 40, 2), vec!["one two three"]);
}

#[test]
fn wrap_marks_the_last_line_when_it_runs_out_of_lines() {
    assert_eq!(
        wrap("alpha beta gamma delta", 11, 2),
        vec!["alpha beta", "gamma delta"],
        "two lines is exactly enough, so nothing is marked"
    );
    assert_eq!(
        wrap("alpha beta gamma delta epsilon", 11, 2),
        vec!["alpha beta", "gamma delt…"],
        "the dropped words leave an ellipsis"
    );
}

#[test]
fn wrap_hard_cuts_a_word_longer_than_the_width() {
    assert_eq!(wrap("abcdefghij", 4, 3), vec!["abcd", "efgh", "ij"]);
    assert_eq!(wrap("abcdefghij", 4, 2), vec!["abcd", "efg…"]);
}

#[test]
fn wrap_of_nothing_is_nothing() {
    assert!(wrap("", 10, 2).is_empty());
    assert!(wrap("   ", 10, 2).is_empty());
    assert!(wrap("text", 0, 2).is_empty());
    assert!(wrap("text", 10, 0).is_empty());
}

#[test]
fn elapsed_keeps_seconds_under_an_hour() {
    assert_eq!(elapsed(Duration::from_secs(45)), "45s");
    assert_eq!(elapsed(Duration::from_secs(239)), "3m59s");
    assert_eq!(elapsed(Duration::from_secs(60)), "1m00s");
}

#[test]
fn elapsed_keeps_minutes_past_an_hour_and_hours_past_a_day() {
    assert_eq!(elapsed(Duration::from_secs(3_840)), "1h04m");
    assert_eq!(elapsed(Duration::from_secs(183_600)), "2d03h");
}

#[test]
fn cells_keeps_a_count_that_fits() {
    assert_eq!(cells(80), 80);
}

#[test]
fn cells_saturates_a_count_above_the_widest_cell_count() {
    assert_eq!(cells(65_536), u16::MAX);
}

#[test]
fn scroll_keeps_an_offset_that_fits() {
    assert_eq!(scroll(12), 12);
}

#[test]
fn scroll_stops_at_the_top() {
    assert_eq!(scroll(-1), 0);
}

#[test]
fn scroll_stops_at_the_furthest_offset() {
    assert_eq!(scroll(70_000), u16::MAX);
}
