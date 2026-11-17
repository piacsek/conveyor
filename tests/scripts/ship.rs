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

#[test]
fn the_first_push_opens_the_draft_with_the_commit_subject_and_the_template() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    template(&work);

    let output = first_ship(dir.path(), "docs(readme): say what it does");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "cargo fmt",
            "gates.sh ",
            "gh pr view topic --json number",
            "gh pr create --draft --assignee @me --title docs(readme): say what it does --body-file .github/pull_request_template.md",
        ]
    );
    assert!(
        stdout(&output).contains("opened draft PR for topic\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn the_title_is_the_first_line_of_a_longer_message() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    template(&work);

    let output = first_ship(dir.path(), "docs: readme\n\nA body\nof two lines");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "cargo fmt",
            "gates.sh ",
            "gh pr view topic --json number",
            "gh pr create --draft --assignee @me --title docs: readme --body-file .github/pull_request_template.md",
        ]
    );
}

#[test]
fn a_missing_template_still_opens_the_draft_with_an_empty_body() {
    let dir = tempfile::tempdir().unwrap();
    ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));

    let output = first_ship(dir.path(), "docs: readme");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "cargo fmt",
            "gates.sh ",
            "gh pr view topic --json number",
            "gh pr create --draft --assignee @me --title docs: readme --body ",
        ]
    );
}

#[test]
fn a_branch_that_has_a_pull_request_opens_no_other() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    template(&work);

    let output = ship(dir.path(), "docs: readme");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        ["cargo fmt", "gates.sh ", "gh pr view topic --json number"]
    );
    assert!(
        !stdout(&output).contains("opened draft PR"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn ship_commits_and_pushes_nothing_when_a_new_file_holds_a_listed_term() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    scrubbing_gates(&work);
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("notes.md"), "notes about zz-secret-term\n").unwrap();

    let output = ship(dir.path(), "docs: notes");

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("scrub-check: denylisted terms found"),
        "{}",
        stderr(&output)
    );
    assert!(!pushed(dir.path()));
}

#[test]
fn ship_commits_and_pushes_nothing_when_what_it_staged_holds_a_listed_term_the_gates_never_saw() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("notes.md"), "notes about zz-secret-term\n").unwrap();

    let output = ship(dir.path(), "docs: notes");

    assert!(!output.status.success(), "{}", stdout(&output));
    assert_eq!(
        calls(dir.path()),
        ["cargo fmt", "gates.sh ", "dev-install.sh "]
    );
    assert_eq!(local_subject(&work), "initial");
    assert!(!pushed(dir.path()));
}

#[test]
fn ship_commits_but_pushes_nothing_when_its_message_holds_a_listed_term() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = first_ship(dir.path(), "fix: port the zz-secret-term workaround");

    assert!(!output.status.success(), "{}", stdout(&output));
    assert_eq!(calls(dir.path()), ["cargo fmt", "gates.sh "]);
    assert_eq!(
        local_subject(&work),
        "fix: port the zz-secret-term workaround"
    );
    assert!(!pushed(dir.path()));
    assert!(
        stderr(&output).contains("git reset --soft HEAD~1"),
        "{}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("--amend"), "{}", stderr(&output));
}

#[test]
fn after_a_refused_message_a_soft_reset_and_a_clean_message_ship() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "changed\n").unwrap();
    first_ship(dir.path(), "fix: port the zz-secret-term workaround");
    git(&work, &["reset", "-q", "--soft", "HEAD~1"]);

    let output = first_ship(dir.path(), "fix: port the workaround");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(pushed_subject(dir.path()), "fix: port the workaround");
}

#[test]
fn after_a_refused_branch_name_a_rename_ships() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    git(&work, &["checkout", "-qb", "zz-secret-term-fix"]);
    fs::write(work.join("README.md"), "changed\n").unwrap();
    let refused = first_ship(dir.path(), "docs: readme");
    git(&work, &["reset", "-q", "--soft", "HEAD~1"]);
    git(&work, &["branch", "-m", "clean-fix"]);

    let output = first_ship(dir.path(), "docs: readme");

    assert!(
        stderr(&refused).contains("git branch -m"),
        "{}",
        stderr(&refused)
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("opened draft PR for clean-fix\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn after_a_refused_merge_message_an_amended_merge_ships() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    merging_main(&work);
    let refused = first_ship(dir.path(), "Merge the zz-secret-term fix");
    git(&work, &["commit", "-q", "--amend", "-m", "Merge main"]);

    let output = first_ship(dir.path(), "Merge main");

    assert!(
        stderr(&refused).contains("git commit --amend"),
        "{}",
        stderr(&refused)
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(pushed_subject(dir.path()), "Merge main");
    assert_eq!(pushed_parents(dir.path()), 2);
}

#[test]
fn ship_pushes_nothing_from_a_branch_named_after_a_listed_term() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    git(&work, &["checkout", "-qb", "zz-secret-term-fix"]);
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = first_ship(dir.path(), "docs: readme");

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(!pushed(dir.path()));
}

#[test]
fn ship_pushes_nothing_when_an_earlier_unpushed_commit_added_a_listed_term_the_tree_no_longer_holds()
 {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "about zz-secret-term\n").unwrap();
    git(&work, &["commit", "-qam", "docs: readme"]);
    fs::write(work.join("README.md"), "readme\n").unwrap();

    let output = first_ship(dir.path(), "docs: readme again");

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(!pushed(dir.path()));
}

#[test]
fn a_clean_ship_with_a_denylist_pushes_and_opens_the_draft() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = first_ship(dir.path(), "docs: readme");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(pushed_subject(dir.path()), "docs: readme");
    assert!(
        stdout(&output).contains("opened draft PR for topic\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn the_template_holds_a_summary_and_the_changes() {
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/pull_request_template.md");

    assert_eq!(
        fs::read_to_string(template).unwrap(),
        "## Summary\n\n## Changes\n"
    );
}

fn ship_sandbox(root: &Path) -> PathBuf {
    let work = repository(root);
    git(root, &["init", "-q", "--bare", "origin.git"]);
    git(&work, &["remote", "add", "origin", "../origin.git"]);
    link_script(&work, "ship.sh");
    link_script(&work, "scrub-check.sh");
    symlink(recorder(), work.join("scripts/gates.sh")).unwrap();
    symlink(recorder(), work.join("scripts/dev-install.sh")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    symlink(recorder(), root.join("bin/cargo")).unwrap();
    symlink(recorder(), root.join("bin/gh")).unwrap();
    work
}

fn ship(root: &Path, message: &str) -> Output {
    shipping(root, message).output().unwrap()
}

fn first_ship(root: &Path, message: &str) -> Output {
    shipping(root, message)
        .env("FAILING_CALL", "pr view")
        .output()
        .unwrap()
}

fn shipping(root: &Path, message: &str) -> Command {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let mut command = Command::new(root.join("work/scripts/ship.sh"));
    isolated(&mut command)
        .arg(message)
        .env("HOME", root.join("home"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .env_remove("FAILING_CALL")
        .env_remove("CONVEYOR_DENYLIST");
    command
}

fn template(work: &Path) {
    fs::create_dir_all(work.join(".github")).unwrap();
    fs::write(
        work.join(".github/pull_request_template.md"),
        "## Summary\n\n## Changes\n",
    )
    .unwrap();
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

fn scrubbing_gates(work: &Path) {
    fs::remove_file(work.join("scripts/gates.sh")).unwrap();
    link_script(work, "gates.sh");
}

fn denylist(root: &Path, terms: &str) {
    let list = root.join("home/.config/conveyor-dev/denylist");
    fs::create_dir_all(list.parent().unwrap()).unwrap();
    fs::write(list, terms).unwrap();
}

fn pushed(root: &Path) -> bool {
    let output = isolated(
        Command::new("git")
            .current_dir(root.join("origin.git"))
            .args(["for-each-ref"]),
    )
    .output()
    .unwrap();
    !stdout(&output).is_empty()
}

fn local_subject(work: &Path) -> String {
    let output = isolated(
        Command::new("git")
            .current_dir(work)
            .args(["log", "-1", "--format=%s"]),
    )
    .output()
    .unwrap();
    stdout(&output).trim().to_string()
}

fn merging_main(work: &Path) {
    fs::write(work.join("README.md"), "topic\n").unwrap();
    git(work, &["commit", "-qam", "topic work"]);
    git(work, &["checkout", "-q", "main"]);
    fs::write(work.join("AGENTS.md"), "main\n").unwrap();
    git(work, &["commit", "-qam", "main work"]);
    git(work, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(work, &["checkout", "-q", "topic"]);
    git(
        work,
        &["merge", "-q", "--no-ff", "--no-commit", "origin/main"],
    );
}

fn pushed_parents(root: &Path) -> usize {
    let output = isolated(
        Command::new("git")
            .current_dir(root.join("origin.git"))
            .args(["log", "-1", "--format=%P", "topic"]),
    )
    .output()
    .unwrap();
    stdout(&output).split_whitespace().count()
}
