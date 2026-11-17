use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::helpers::{stderr, stdout, which};

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

fn feeds_binary(paths: &str) -> Output {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/feeds-binary.sh");
    let mut child = Command::new(which("bash"))
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(paths.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
