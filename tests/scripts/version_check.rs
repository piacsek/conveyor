use std::fs;
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

#[test]
fn an_edited_changelog_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "CHANGELOG.md",
        "# Changelog\n\n## v0.3.2 (2026-11-02)\n- Fixed the queue.\n\n## v0.3.1 (2026-11-01)\n- Fixed the builds.\n",
    );

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "CHANGELOG.md is changed: leave it as main has it, the release workflow writes it from the fragments in changes/ (add one there instead)\n"
    );
}

#[test]
fn a_changelog_main_changed_after_the_branch_was_cut_is_not_the_pull_requests_change() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    git(&work, &["checkout", "-q", "main"]);
    committed(
        &work,
        "CHANGELOG.md",
        "# Changelog\n\n## v0.3.1 (2026-11-01)\n- Fixed the builds.\n- Fixed the footer.\n",
    );

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn an_edited_fragment_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_fragment(dir.path());
    fragment(&work, "merged", "kind: feat\n\n- A new key.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/merged.md is edited: a fragment on main never changes, so leave it as it is and put the correction in a new file changes/<name>.md\n"
    );
}

#[test]
fn a_deleted_fragment_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_fragment(dir.path());
    git(&work, &["rm", "-q", "changes/merged.md"]);
    git(&work, &["commit", "-qm", "drop a fragment"]);

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/merged.md is deleted: a fragment on main is never deleted, so restore it as main has it\n"
    );
}

#[test]
fn a_renamed_fragment_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_fragment(dir.path());
    git(&work, &["mv", "changes/merged.md", "changes/renamed.md"]);
    git(&work, &["commit", "-qm", "rename a fragment"]);

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/merged.md is renamed to changes/renamed.md: a fragment on main is never renamed, so restore changes/merged.md and add new text as a new file changes/<name>.md\n"
    );
}

#[test]
fn a_file_renamed_into_changes_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    git(&work, &["checkout", "-q", "main"]);
    committed(&work, "docs/note.md", "kind: fix\n\n- Fixed the queue.\n");
    git(&work, &["checkout", "-qB", "pull"]);
    fs::create_dir_all(work.join("changes")).unwrap();
    git(&work, &["mv", "docs/note.md", "changes/note.md"]);
    git(&work, &["commit", "-qm", "move a note into changes"]);

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "docs/note.md is renamed to changes/note.md: a fragment is only ever added, so restore docs/note.md and add changes/note.md as a new file\n"
    );
}

#[test]
fn an_added_fragment_with_an_unknown_kind_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    fragment(&work, "topic", "kind: major\n\n- A new config format.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md must start with `kind: fix`, `kind: feat` or `kind: internal`\n"
    );
}

#[test]
fn an_added_file_in_a_folder_under_changes_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "changes/fix/topic.md",
        "kind: fix\n\n- Fixed the queue.\n",
    );

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/fix/topic.md is not a fragment: a fragment is a file changes/<name>.md\n"
    );
}

#[test]
fn an_added_fragment_repeating_a_released_bullet_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the builds.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md repeats a bullet of v0.3.1: Fixed the builds.\n"
    );
}

fn main_with_a_fragment(root: &Path) -> PathBuf {
    let work = main_with_a_release(root);
    git(&work, &["checkout", "-q", "main"]);
    fragment(&work, "merged", "kind: fix\n\n- Fixed the queue.\n");
    git(&work, &["checkout", "-qB", "pull"]);
    work
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
