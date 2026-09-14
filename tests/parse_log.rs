use conveyor::model::log::tail;

#[test]
fn tail_keeps_the_last_lines_without_the_job_step_prefix_or_ansi_colour() {
    let mut text = String::new();
    for i in 1..=45 {
        text.push_str(&format!("check\tRun cargo test\t\x1b[31mline {i}\x1b[0m\n"));
    }
    text.push_str("\n\n");

    let lines = tail(&text, 40);

    assert_eq!(lines.len(), 40);
    assert_eq!(lines[0], "line 6");
    assert_eq!(lines[39], "line 45");
}

#[test]
fn tail_leaves_lines_without_the_prefix_alone_and_an_empty_log_empty() {
    assert_eq!(tail("no tabs here\n", 40), vec!["no tabs here"]);
    assert_eq!(
        tail("a\tb\n", 40),
        vec!["a\tb"],
        "two fields are not a prefix"
    );
    assert_eq!(
        tail(
            "check\tRun\t2026-09-14T16:09:52.5114315Z cargo test failed\n",
            40
        ),
        vec!["cargo test failed"],
        "the line's own timestamp goes too"
    );
    assert!(tail("", 40).is_empty());
    assert!(tail("\n\n", 40).is_empty());
}
