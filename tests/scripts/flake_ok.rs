use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{stderr, stdout, which};

#[test]
fn both_legs_passing_passes() {
    let output = flake_ok("success", "true", "success");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "the flake builds\n");
}

#[test]
fn a_failed_leg_fails() {
    let output = flake_ok("success", "true", "failure");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "the flake build ended in failure\n");
}

#[test]
fn a_cancelled_leg_fails() {
    let output = flake_ok("success", "true", "cancelled");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "the flake build ended in cancelled\n");
}

#[test]
fn unchanged_inputs_pass_without_a_build() {
    let output = flake_ok("success", "false", "skipped");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "no input of the flake changed\n");
}

#[test]
fn a_build_that_ran_after_the_paths_job_failed_passes() {
    let output = flake_ok("failure", "", "success");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "the flake builds\n");
}

#[test]
fn a_build_that_failed_after_the_paths_job_failed_fails() {
    let output = flake_ok("failure", "", "failure");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "the flake build ended in failure\n");
}

#[test]
fn a_run_cancelled_before_the_build_fails() {
    let output = flake_ok("cancelled", "", "skipped");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "the flake build was skipped, but the paths job ended in cancelled with build=\n"
    );
}

#[test]
fn a_skipped_build_the_paths_job_asked_for_fails() {
    let output = flake_ok("success", "true", "skipped");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "the flake build was skipped, but the paths job ended in success with build=true\n"
    );
}

#[test]
fn a_skipped_build_after_a_failed_paths_job_fails_even_when_it_said_false() {
    let output = flake_ok("failure", "false", "skipped");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "the flake build was skipped, but the paths job ended in failure with build=false\n"
    );
}

#[test]
fn no_arguments_fail() {
    let output = Command::new(which("bash")).arg(script()).output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "the flake build ended in no result\n");
}

fn flake_ok(paths: &str, build: &str, flake: &str) -> Output {
    Command::new(which("bash"))
        .arg(script())
        .args([paths, build, flake])
        .output()
        .unwrap()
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/flake-ok.sh")
}
