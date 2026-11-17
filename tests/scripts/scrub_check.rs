use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{git, isolated, link_script, repository, stderr, stdout};

#[test]
fn a_listed_term_only_in_the_index_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "about zz-secret-term\n").unwrap();
    git(&work, &["add", "README.md"]);
    fs::write(work.join("README.md"), "readme\n").unwrap();

    let output = scrub(&work, Some(&list));

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("README.md:1:about zz-secret-term"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_listed_term_only_in_an_ignored_file_leaves_the_scrub_clean() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::create_dir_all(work.join("target")).unwrap();
    fs::write(work.join("target/notes.md"), "about zz-secret-term\n").unwrap();

    let output = scrub(&work, Some(&list));

    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "scrub-check: clean\n");
}

#[test]
fn a_denylist_git_grep_cannot_use_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "");

    let output = scrub(&work, Some(&list));

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("scrub-check: git grep --untracked failed"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_denylist_named_by_the_variable_that_is_missing_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = dir.path().join("missing");

    let output = scrub(&work, Some(&list));

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&format!(
            "scrub-check: {} is not a readable file",
            list.display()
        )),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_default_denylist_that_is_a_directory_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    fs::create_dir_all(default_denylist(dir.path())).unwrap();

    let output = scrub(&work, None);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("/home/.config/conveyor-dev/denylist is not a readable file"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_default_denylist_that_is_a_dangling_link_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = default_denylist(dir.path());
    fs::create_dir_all(list.parent().unwrap()).unwrap();
    symlink(dir.path().join("gone"), &list).unwrap();

    let output = scrub(&work, None);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("/home/.config/conveyor-dev/denylist is not a readable file"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn no_default_denylist_skips_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());

    let output = scrub(&work, None);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("/home/.config/conveyor-dev/denylist, skipped\n"),
        "{}",
        stdout(&output)
    );
}

fn scrub_sandbox(root: &Path) -> PathBuf {
    let work = repository(root);
    link_script(&work, "scrub-check.sh");
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "the scrub"]);
    work
}

fn denylist(root: &Path, terms: &str) -> PathBuf {
    let list = root.join("denylist");
    fs::write(&list, terms).unwrap();
    list
}

fn scrub(work: &Path, list: Option<&Path>) -> Output {
    let mut command = Command::new(work.join("scripts/scrub-check.sh"));
    isolated(&mut command)
        .env("HOME", work.parent().unwrap().join("home"))
        .env_remove("CONVEYOR_DENYLIST");
    if let Some(list) = list {
        command.env("CONVEYOR_DENYLIST", list);
    }
    command.output().unwrap()
}

fn default_denylist(root: &Path) -> PathBuf {
    root.join("home/.config/conveyor-dev/denylist")
}
