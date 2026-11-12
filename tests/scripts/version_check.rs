use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{
    committed, fragment, git, isolated, link_script, released, stderr, stdout, which,
};

#[test]
fn a_pull_request_that_adds_a_well_formed_fragment_passes() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "the pull request leaves the version to the release workflow\n"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_package_version_other_than_0_0_0_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion = \"0.4.0\"\n\n[dependencies]\nserde = { version = \"1\" }\n",
    );
    fragment(&work, "topic", "kind: feat\n\n- A new key.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Cargo.toml sets the package version to 0.4.0: put back `version = \"0.0.0\"`, only the release workflow picks a version (from the kinds in changes/)\n"
    );
}

#[test]
fn a_lock_entry_other_than_0_0_0_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.3.2\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Cargo.lock sets the conveyor entry to 0.3.2: put back `version = \"0.0.0\"` under `name = \"conveyor\"`, only the release workflow picks a version (from the kinds in changes/)\n"
    );
}

#[test]
fn a_call_without_a_base_and_a_head_prints_the_usage() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());

    let output = isolated(
        Command::new(which("bash"))
            .current_dir(&work)
            .arg(work.join("scripts/version-check.sh"))
            .arg("main"),
    )
    .output()
    .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "usage: scripts/version-check.sh <base> <head>\n"
    );
}

#[test]
fn a_head_that_is_not_a_commit_here_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());

    let output = isolated(
        Command::new(which("bash"))
            .current_dir(&work)
            .arg(work.join("scripts/version-check.sh"))
            .args(["main", "8c1f2a7d5e3b4c6a9f0e1d2c3b4a5f6e7d8c9b0a"]),
    )
    .output()
    .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "8c1f2a7d5e3b4c6a9f0e1d2c3b4a5f6e7d8c9b0a is not a commit here: fetch the pull request first (git fetch origin pull/<number>/head)\n"
    );
}

fn main_with_a_release(root: &Path) -> PathBuf {
    let work = released(root, "v0.3.1");
    link_script(&work, "version-check.sh");
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\n\n[dependencies]\nserde = { version = \"1\" }\n",
    );
    committed(
        &work,
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.0.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    );
    committed(
        &work,
        "CHANGELOG.md",
        "# Changelog\n\n## v0.3.1 (2026-11-01)\n- Fixed the builds.\n",
    );
    git(&work, &["tag", "-f", "v0.3.1"]);
    git(&work, &["checkout", "-qB", "main"]);
    git(&work, &["checkout", "-qb", "pull"]);
    work
}

fn version_check(work: &Path) -> Output {
    git(work, &["checkout", "-q", "main"]);
    isolated(
        Command::new(which("bash"))
            .current_dir(work)
            .arg(work.join("scripts/version-check.sh"))
            .args(["main", "pull"]),
    )
    .output()
    .unwrap()
}
