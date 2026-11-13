use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{git, isolated, link_script, repository, stderr, stdout, which};

const SKIPPABLE: &str = "nothing on this branch reaches the script tests\n";

#[test]
fn a_branch_of_sources_other_tests_and_docs_can_skip_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::write(work.join("src/main.rs"), "fn main() { }\n").unwrap();
    git(&work, &["commit", "-qam", "source"]);
    fs::write(work.join("README.md"), "changed\n").unwrap();
    fs::write(work.join("tests/cli.rs"), "").unwrap();
    fs::create_dir_all(work.join("docs")).unwrap();
    fs::write(work.join("docs/note.md"), "note\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), SKIPPABLE);
}

#[test]
fn an_untracked_script_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::write(work.join("scripts/another.sh"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "scripts/another.sh needs the script tests\n"
    );
}

#[test]
fn a_change_to_the_script_tests_themselves_needs_them() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::create_dir_all(work.join("tests/scripts")).unwrap();
    fs::write(work.join("tests/scripts/new.rs"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "tests/scripts/new.rs needs the script tests\n"
    );
}

#[test]
fn a_test_fixture_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::create_dir_all(work.join("tests/fixtures")).unwrap();
    fs::write(work.join("tests/fixtures/record-call.sh"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "tests/fixtures/record-call.sh needs the script tests\n"
    );
}

#[test]
fn a_workflow_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::create_dir_all(work.join(".github/workflows")).unwrap();
    fs::write(work.join(".github/workflows/version.yml"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        ".github/workflows/version.yml needs the script tests\n"
    );
}

#[test]
fn a_fragment_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::create_dir_all(work.join("changes")).unwrap();
    fs::write(work.join("changes/topic.md"), "kind: fix\n\n- Fixed.\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "changes/topic.md needs the script tests\n");
}

#[test]
fn the_manifest_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::write(work.join("Cargo.toml"), "[package]\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "Cargo.toml needs the script tests\n");
}

#[test]
fn the_lock_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::write(work.join("Cargo.lock"), "version = 4\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "Cargo.lock needs the script tests\n");
}

#[test]
fn the_changelog_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::write(work.join("CHANGELOG.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "CHANGELOG.md needs the script tests\n");
}

#[test]
fn a_script_change_committed_earlier_on_the_branch_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    fs::write(work.join("scripts/another.sh"), "").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "script"]);
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "scripts/another.sh needs the script tests\n"
    );
}

#[test]
fn a_branch_with_no_known_main_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = checked_repository(dir.path());
    git(&work, &["update-ref", "-d", "refs/remotes/origin/main"]);

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "no merge base with origin/main\n");
}

#[test]
fn a_directory_outside_any_repository_needs_the_script_tests() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    link_script(&work, "script-tests-skippable.sh");

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "no merge base with origin/main\n");
}

fn checked_repository(root: &Path) -> PathBuf {
    let work = repository(root);
    link_script(&work, "script-tests-skippable.sh");
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "the check"]);
    git(&work, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    work
}

fn skippable(root: &Path) -> Output {
    isolated(
        Command::new(which("bash"))
            .arg(root.join("work/scripts/script-tests-skippable.sh"))
            .env("GIT_CEILING_DIRECTORIES", root),
    )
    .output()
    .unwrap()
}
