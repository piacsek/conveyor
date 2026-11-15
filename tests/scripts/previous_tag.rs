use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{calls, isolated, link_script, stderr, stdout, which};

#[test]
fn the_highest_lower_release_tag_is_the_previous_one() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    tags(dir.path(), "v0.3.1\nv0.3.0\nv0.2.4\nv0.1.0\n");

    let output = previous(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "previous_tag=v0.3.0\n");
}

#[test]
fn the_tags_are_read_from_every_page_of_the_repository() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    tags(dir.path(), "v0.3.1\nv0.3.0\n");

    previous(dir.path());

    assert_eq!(
        calls(dir.path())[0],
        "gh api --paginate repos/{owner}/{repo}/tags --jq .[].name"
    );
}

#[test]
fn tags_are_compared_in_version_order_not_text_order() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    tags(dir.path(), "v0.3.1\nv0.10.0\nv0.9.0\n");

    let output = previous_of(dir.path(), "v0.11.0", "no call fails", "");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "previous_tag=v0.10.0\n");
}

#[test]
fn higher_release_tags_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    tags(dir.path(), "v1.0.0\nv0.4.0\nv0.3.1\nv0.2.9\n");

    let output = previous(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "previous_tag=v0.2.9\n");
}

#[test]
fn the_lowest_release_tag_has_no_previous_one() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    tags(dir.path(), "v0.4.0\nv0.3.1\n");

    let output = previous(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "previous_tag=\n");
}

#[test]
fn names_that_are_not_release_tags_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    tags(
        dir.path(),
        "v0.3.1\nv0.3.0-test\nv1-test\nnightly\nv0.2.x\nv0.2.0\n",
    );

    let output = previous(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "previous_tag=v0.2.0\n");
}

#[test]
fn a_tag_missing_from_the_list_still_gets_the_highest_below_it() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    tags(dir.path(), "v0.4.0\nv0.3.0\nv0.2.0\n");

    let output = previous(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "previous_tag=v0.3.0\n");
}

#[test]
fn tags_that_cannot_be_read_stop_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());

    let output = previous_of(
        dir.path(),
        "v0.3.1",
        "api --paginate",
        "HTTP 502: Bad Gateway",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "the tags below v0.3.1 could not be read: HTTP 502: Bad Gateway\n"
    );
}

fn previous(root: &Path) -> Output {
    previous_of(root, "v0.3.1", "no call fails", "")
}

fn previous_of(root: &Path, tag: &str, failing_call: &str, failure: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(which("bash")))
        .arg(root.join("work/scripts/previous-tag.sh"))
        .arg(tag)
        .current_dir(root.join("work"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .env("REPLIES", root.join("replies"))
        .env("FAILING_CALL", failing_call)
        .env("FAILING_STDERR", failure)
        .output()
        .unwrap()
}

fn tags(root: &Path, names: &str) {
    fs::write(root.join("replies/tags"), names).unwrap();
}

fn sandbox(root: &Path) {
    link_script(&root.join("work"), "previous-tag.sh");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("replies")).unwrap();
    symlink(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reply-call.sh"),
        root.join("bin/gh"),
    )
    .unwrap();
}
