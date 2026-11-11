use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{calls, link_script, recorder, stderr, which};

#[test]
fn gates_run_every_test_in_one_pass_when_tmux_is_installed() {
    let dir = tempfile::tempdir().unwrap();
    let bin = gates_sandbox(dir.path());
    symlink(which("true"), bin.join("tmux")).unwrap();

    let output = gates(dir.path());

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "scrub-check.sh ",
            "cargo fmt --check",
            "cargo clippy --all-targets -- -D warnings",
            "cargo test -- --include-ignored",
        ]
    );
}

#[test]
fn gates_without_tmux_run_the_plain_tests_and_say_what_they_skipped() {
    let dir = tempfile::tempdir().unwrap();
    gates_sandbox(dir.path());

    let output = gates(dir.path());

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "scrub-check.sh ",
            "cargo fmt --check",
            "cargo clippy --all-targets -- -D warnings",
            "cargo test",
        ]
    );
    assert!(
        stderr(&output).contains("skipping the ignored e2e tests: tmux is not installed"),
        "{}",
        stderr(&output)
    );
}

fn gates_sandbox(root: &Path) -> PathBuf {
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    symlink(which("bash"), bin.join("bash")).unwrap();
    symlink(which("dirname"), bin.join("dirname")).unwrap();
    symlink(recorder(), bin.join("cargo")).unwrap();
    link_script(root, "gates.sh");
    symlink(recorder(), root.join("scripts/scrub-check.sh")).unwrap();
    bin
}

fn gates(root: &Path) -> Output {
    Command::new(root.join("scripts/gates.sh"))
        .env_clear()
        .env("PATH", root.join("bin"))
        .env("CALLS_LOG", root.join("calls.log"))
        .output()
        .unwrap()
}
