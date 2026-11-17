use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{committed, isolated, link_script, repository, stderr, stdout, which};

const BUILD: &str = "build=true\n";
const SKIP: &str = "build=false\n";

#[test]
fn a_source_change_since_the_base_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "src/main.rs", "fn main() { }\n");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn a_change_the_flake_does_not_read_skips_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), SKIP);
}

#[test]
fn a_source_file_with_a_non_ascii_name_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "src/caf\u{e9}.rs", "\n");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn a_source_file_with_a_space_in_its_name_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "src/a file.rs", "\n");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn an_input_renamed_away_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    fs::rename(work.join("CHANGELOG.md"), work.join("docs.md")).unwrap();
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn every_commit_since_the_base_counts() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "flake.lock", "{}\n");
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &["HEAD~2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn a_deleted_input_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    fs::remove_file(work.join("CHANGELOG.md")).unwrap();
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn a_base_that_is_not_a_commit_here_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &["0123456789abcdef0123456789abcdef01234567"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn no_base_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn building_because_it_could_not_decide_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &["0123456789abcdef0123456789abcdef01234567"]);

    assert_eq!(
        stderr(&output),
        "flake-build-needed: could not list the paths changed since 0123456789abcdef0123456789abcdef01234567, so the flake builds\n"
    );
}

#[test]
fn deciding_from_the_paths_says_nothing_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "src/main.rs", "fn main() { }\n");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(stderr(&output), "");
}

#[test]
fn an_empty_base_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = deciding_repository(dir.path());
    committed(&work, "README.md", "changed\n");

    let output = build_needed(&work, &[""]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

#[test]
fn a_directory_outside_any_repository_needs_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    link_script(&work, "flake-inputs.sh");
    link_script(&work, "flake-build-needed.sh");

    let output = build_needed(&work, &["HEAD^1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), BUILD);
}

fn deciding_repository(root: &Path) -> PathBuf {
    let work = repository(root);
    link_script(&work, "flake-inputs.sh");
    link_script(&work, "flake-build-needed.sh");
    committed(&work, "flake.lock", "{ }\n");
    work
}

fn build_needed(work: &Path, args: &[&str]) -> Output {
    isolated(
        Command::new(which("bash"))
            .arg(work.join("scripts/flake-build-needed.sh"))
            .args(args)
            .current_dir(work)
            .env("GIT_CEILING_DIRECTORIES", work.parent().unwrap()),
    )
    .output()
    .unwrap()
}
