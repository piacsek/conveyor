use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{
    another_build, build_in, git, isolated, link_conveyor_dev, repository, stderr, stdout, which,
};

#[test]
fn a_branch_that_feeds_nothing_into_the_binary_can_skip_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("README.md"), "changed\n").unwrap();
    fs::write(work.join("tests/new.rs"), "").unwrap();
    git(&work, &["commit", "-qam", "docs"]);
    fs::write(work.join("AGENTS.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "nothing on this branch feeds the binary, and conveyor-dev is another worktree's build\n"
    );
}

#[test]
fn an_agents_file_next_to_the_sources_can_skip_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::create_dir_all(work.join("src/app")).unwrap();
    fs::write(work.join("src/app/AGENTS.md"), "agents\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "nothing on this branch feeds the binary, and conveyor-dev is another worktree's build\n"
    );
}

#[test]
fn a_fragment_under_changes_can_skip_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::create_dir_all(work.join("changes")).unwrap();
    fs::write(work.join("changes/topic.md"), "kind: fix\n\n- Fixed.\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "nothing on this branch feeds the binary, and conveyor-dev is another worktree's build\n"
    );
}

#[test]
fn an_untracked_source_file_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("src/new.rs"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/new.rs feeds the binary\n");
}

#[test]
fn the_changelog_is_bundled_so_editing_it_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("CHANGELOG.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "CHANGELOG.md feeds the binary\n");
}

#[test]
fn a_path_nobody_listed_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("build.rs"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "build.rs feeds the binary\n");
}

#[test]
fn a_source_change_committed_earlier_on_the_branch_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("src/main.rs"), "fn main() { }\n").unwrap();
    git(&work, &["commit", "-qam", "source"]);
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/main.rs feeds the binary\n");
}

#[test]
fn a_missing_conveyor_dev_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    repository(dir.path());

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "conveyor-dev is missing or dangling\n");
}

#[test]
fn a_dangling_conveyor_dev_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    repository(dir.path());
    link_conveyor_dev(dir.path(), &dir.path().join("removed/conveyor"));

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "conveyor-dev is missing or dangling\n");
}

#[test]
fn a_conveyor_dev_built_here_always_goes_through_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &build_in(&work));

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "conveyor-dev is this worktree's build\n");
}

#[test]
fn a_branch_with_no_known_main_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    git(&work, &["update-ref", "-d", "refs/remotes/origin/main"]);

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "no merge base with origin/main\n");
}

fn skippable(root: &Path) -> Output {
    isolated(Command::new(which("bash")).arg(root.join("work/scripts/dev-install-skippable.sh")))
        .env("HOME", root.join("home"))
        .output()
        .unwrap()
}
