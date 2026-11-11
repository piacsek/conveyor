use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{
    another_build, calls, git, isolated, link_conveyor_dev, link_script, recorder, repository,
    stderr, stdout,
};

#[test]
fn ship_skips_the_dev_install_and_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = ship(dir.path(), "docs: readme");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        ["cargo fmt", "gates.sh ", "gh pr view topic --json number"]
    );
    assert!(
        stdout(&output).contains(
            "skipping dev-install: nothing on this branch feeds the binary, and conveyor-dev is another worktree's build\n"
        ),
        "{}",
        stdout(&output)
    );
    assert_eq!(pushed_subject(dir.path()), "docs: readme");
}

#[test]
fn ship_runs_the_dev_install_when_the_commit_feeds_the_binary() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("src/main.rs"), "fn main() { }\n").unwrap();

    let output = ship(dir.path(), "fix: main");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "cargo fmt",
            "gates.sh ",
            "dev-install.sh ",
            "gh pr view topic --json number",
        ]
    );
    assert!(
        !stdout(&output).contains("skipping dev-install"),
        "{}",
        stdout(&output)
    );
    assert_eq!(pushed_subject(dir.path()), "fix: main");
}

fn ship_sandbox(root: &Path) -> PathBuf {
    let work = repository(root);
    git(root, &["init", "-q", "--bare", "origin.git"]);
    git(&work, &["remote", "add", "origin", "../origin.git"]);
    link_script(&work, "ship.sh");
    symlink(recorder(), work.join("scripts/gates.sh")).unwrap();
    symlink(recorder(), work.join("scripts/dev-install.sh")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    symlink(recorder(), root.join("bin/cargo")).unwrap();
    symlink(recorder(), root.join("bin/gh")).unwrap();
    work
}

fn ship(root: &Path, message: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(root.join("work/scripts/ship.sh")))
        .arg(message)
        .env("HOME", root.join("home"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .output()
        .unwrap()
}

fn pushed_subject(root: &Path) -> String {
    let output = isolated(
        Command::new("git")
            .current_dir(root.join("origin.git"))
            .args(["log", "-1", "--format=%s", "topic"]),
    )
    .output()
    .unwrap();
    stdout(&output).trim().to_string()
}
