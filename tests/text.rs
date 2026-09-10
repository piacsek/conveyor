use conveyor::text::wrap;

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
