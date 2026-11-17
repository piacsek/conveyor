use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
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
    let list = denylist(dir.path(), "\n");

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

#[test]
fn a_listed_term_only_in_the_name_of_an_untracked_file_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("-ZZ-Secret-Term notes.md"), "clean\n").unwrap();

    let output = scrub(&work, Some(&list));

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("-ZZ-Secret-Term notes.md\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_listed_term_only_in_the_name_of_a_staged_file_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::create_dir_all(work.join("docs")).unwrap();
    fs::write(work.join("docs/zz-secret-term.md"), "clean\n").unwrap();
    git(&work, &["add", "docs/zz-secret-term.md"]);

    let output = scrub(&work, Some(&list));

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("docs/zz-secret-term.md\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_listed_term_only_in_the_name_of_an_ignored_file_leaves_the_scrub_clean() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::create_dir_all(work.join("target")).unwrap();
    fs::write(work.join("target/zz-secret-term.md"), "clean\n").unwrap();

    let output = scrub(&work, Some(&list));

    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "scrub-check: clean\n");
}

#[test]
fn an_outgoing_symlink_whose_target_holds_a_listed_term_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    symlink("../zz-secret-term/notes", work.join("link")).unwrap();
    git(&work, &["add", "link"]);
    git(&work, &["commit", "-qm", "a link"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("added line: ../zz-secret-term/notes\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_listed_term_only_in_commits_a_remote_already_has_passes_the_outgoing_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "about zz-secret-term\n").unwrap();
    git(&work, &["commit", "-qam", "zz-secret-term"]);
    git(&work, &["update-ref", "refs/remotes/origin/topic", "HEAD"]);

    let output = scrub_outgoing(&work, &list);

    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "scrub-check: clean\n");
}

#[test]
fn an_outgoing_scrub_git_log_cannot_read_fails() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    git(&work, &["checkout", "-q", "--orphan", "empty"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("scrub-check: git log failed"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn removing_a_file_that_holds_a_listed_term_a_remote_has_passes_the_outgoing_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    pushed_file(&work, "notes.md", "about zz-secret-term\n");
    git(&work, &["rm", "-q", "notes.md"]);
    git(&work, &["commit", "-qm", "drop the notes"]);

    let output = scrub_outgoing(&work, &list);

    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "scrub-check: clean\n");
}

#[test]
fn deleting_a_line_that_holds_a_listed_term_a_remote_has_passes_the_outgoing_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    pushed_file(&work, "notes.md", "one\nabout zz-secret-term\nthree\n");
    fs::write(work.join("notes.md"), "one\nthree\n").unwrap();
    git(&work, &["commit", "-qam", "drop the line"]);

    let output = scrub_outgoing(&work, &list);

    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "scrub-check: clean\n");
}

#[test]
fn deleting_a_file_named_after_a_listed_term_a_remote_has_passes_the_outgoing_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    pushed_file(&work, "zz-secret-term.md", "notes\n");
    git(&work, &["rm", "-q", "zz-secret-term.md"]);
    git(&work, &["commit", "-qm", "drop the notes"]);

    let output = scrub_outgoing(&work, &list);

    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "scrub-check: clean\n");
}

#[test]
fn an_outgoing_added_line_that_holds_a_listed_term_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "about zz-secret-term\n").unwrap();
    git(&work, &["commit", "-qam", "docs"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains(&format!(
            "scrub-check: commit {} (HEAD) added line: about zz-secret-term\n",
            short(&work, "HEAD")
        )),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_outgoing_path_that_holds_a_listed_term_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("zz-secret-term.md"), "").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "an empty file"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains(&format!(
            "scrub-check: commit {} (HEAD) path: zz-secret-term.md\n",
            short(&work, "HEAD")
        )),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_outgoing_rename_to_a_listed_name_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    git(&work, &["mv", "README.md", "zz-secret-term.md"]);
    git(&work, &["commit", "-qm", "rename"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("path: zz-secret-term.md\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_outgoing_commit_message_that_holds_a_listed_term_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    git(
        &work,
        &["commit", "-q", "--allow-empty", "-m", "port zz-secret-term"],
    );

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains(&format!(
            "scrub-check: commit {} (HEAD) message: port zz-secret-term\n",
            short(&work, "HEAD")
        )),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_earlier_outgoing_commit_is_named_without_head() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    git(
        &work,
        &["commit", "-q", "--allow-empty", "-m", "port zz-secret-term"],
    );
    let earlier = short(&work, "HEAD");
    git(&work, &["commit", "-q", "--allow-empty", "-m", "later"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains(&format!(
            "scrub-check: commit {earlier} message: port zz-secret-term\n"
        )),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_outgoing_branch_name_that_holds_a_listed_term_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    git(&work, &["checkout", "-qb", "zz-secret-term-fix"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("scrub-check: branch name: zz-secret-term-fix\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_outgoing_merge_that_resolves_a_conflict_with_a_listed_term_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-secret-term\n");
    fs::write(work.join("README.md"), "topic\n").unwrap();
    git(&work, &["commit", "-qam", "topic side"]);
    git(&work, &["checkout", "-q", "main"]);
    fs::write(work.join("README.md"), "main\n").unwrap();
    git(&work, &["commit", "-qam", "main side"]);
    git(&work, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&work, &["checkout", "-q", "topic"]);
    let merge =
        isolated(
            Command::new("git")
                .current_dir(&work)
                .args(["merge", "-q", "origin/main"]),
        )
        .output()
        .unwrap();
    assert!(!merge.status.success());
    fs::write(work.join("README.md"), "resolved zz-secret-term\n").unwrap();
    git(&work, &["commit", "-qam", "merge main"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("(HEAD) added line: resolved zz-secret-term\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_listed_term_that_is_part_of_a_blob_hash_passes_the_outgoing_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    fs::write(work.join("README.md"), "changed\n").unwrap();
    git(&work, &["commit", "-qam", "docs"]);
    let list = denylist(dir.path(), &format!("{}\n", short(&work, "HEAD:README.md")));

    let output = scrub_outgoing(&work, &list);

    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "scrub-check: clean\n");
}

#[test]
fn an_empty_denylist_fails_the_outgoing_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "");

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&format!("scrub-check: {} is empty", list.display())),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_default_denylist_under_a_directory_without_search_permission_fails_the_scrub() {
    if running_as_root() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let parent = default_denylist(dir.path()).parent().unwrap().to_path_buf();
    fs::create_dir_all(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o600)).unwrap();

    let output = scrub(&work, None);

    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&format!(
            "scrub-check: {} is not a readable file",
            default_denylist(dir.path()).display()
        )),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_default_denylist_whose_parent_is_a_file_fails_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let parent = default_denylist(dir.path()).parent().unwrap().to_path_buf();
    fs::create_dir_all(parent.parent().unwrap()).unwrap();
    fs::write(&parent, "").unwrap();

    let output = scrub(&work, None);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&format!(
            "scrub-check: {} is not a readable file",
            default_denylist(dir.path()).display()
        )),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_non_ascii_listed_name_renamed_away_before_head_fails_the_outgoing_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let work = scrub_sandbox(dir.path());
    let list = denylist(dir.path(), "zz-s\u{e9}cret\n");
    fs::write(work.join("zz-s\u{e9}cret.md"), "").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "notes"]);
    git(&work, &["mv", "zz-s\u{e9}cret.md", "notes.md"]);
    git(&work, &["commit", "-qm", "rename"]);

    let output = scrub_outgoing(&work, &list);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("path: zz-s\u{e9}cret.md\n"),
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

fn scrub_outgoing(work: &Path, list: &Path) -> Output {
    isolated(&mut Command::new(work.join("scripts/scrub-check.sh")))
        .arg("--outgoing")
        .env("HOME", work.parent().unwrap().join("home"))
        .env("CONVEYOR_DENYLIST", list)
        .output()
        .unwrap()
}

fn pushed_file(work: &Path, path: &str, text: &str) {
    fs::write(work.join(path), text).unwrap();
    git(work, &["add", "-A"]);
    git(work, &["commit", "-qm", "on a remote"]);
    git(work, &["update-ref", "refs/remotes/origin/topic", "HEAD"]);
}

fn short(work: &Path, rev: &str) -> String {
    let output =
        isolated(
            Command::new("git")
                .current_dir(work)
                .args(["rev-parse", "--short", rev]),
        )
        .output()
        .unwrap();
    stdout(&output).trim().to_string()
}

fn running_as_root() -> bool {
    let output = Command::new("id").arg("-u").output().unwrap();
    stdout(&output).trim() == "0"
}
