use std::process::Output;

use crate::helpers::{piped, stderr, stdout};

#[test]
fn only_the_paths_that_feed_the_binary_are_printed() {
    let output = feeds_binary(
        "src/main.rs\nREADME.md\ntests/cli.rs\nCargo.toml\nsrc/app/AGENTS.md\nbuild.rs\n",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/main.rs\nCargo.toml\nbuild.rs\n");
}

#[test]
fn paths_that_feed_nothing_print_nothing_and_succeed() {
    let output = feeds_binary("README.md\n\nplans/phase-1.md\n");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_fragment_under_changes_does_not_feed_the_binary() {
    let output = feeds_binary("changes/topic.md\nsrc/changes/topic.md\n");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/changes/topic.md\n");
}

#[test]
fn a_claude_skill_does_not_feed_the_binary() {
    let output = feeds_binary(".claude/skills/review/SKILL.md\nsrc/.claude/topic.md\n");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/.claude/topic.md\n");
}

fn feeds_binary(paths: &str) -> Output {
    piped("feeds-binary.sh", paths)
}
