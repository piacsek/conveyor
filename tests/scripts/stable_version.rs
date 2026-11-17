use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{isolated, link_script, stderr, stdout, which};

#[test]
fn a_stable_branch_behind_the_newest_complete_release_moves_to_it() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    complete(dir.path(), "v0.3.2");
    stable(dir.path(), "0.3.1");

    let output = stable_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "version=0.3.2\n");
}

#[test]
fn a_stable_branch_at_the_newest_complete_release_stays() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    complete(dir.path(), "v0.3.2");
    stable(dir.path(), "0.3.2");

    let output = stable_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_stable_branch_ahead_of_every_complete_release_never_goes_back() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    short(dir.path(), "v0.3.2");
    stable(dir.path(), "0.3.2");

    let output = stable_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_newest_release_still_a_draft_is_passed_over() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    fs::write(dir.path().join("replies/v0.3.2"), format!("true\n{ASSETS}")).unwrap();
    stable(dir.path(), "0.3.0");

    let output = stable_version(dir.path());

    assert_eq!(stdout(&output), "version=0.3.1\n");
}

#[test]
fn a_stable_branch_on_the_unreleased_main_moves_to_the_newest_complete_release() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    stable(dir.path(), "0.0.0");

    let output = stable_version(dir.path());

    assert_eq!(stdout(&output), "version=0.3.1\n");
}

#[test]
fn no_stable_branch_yet_gets_the_newest_complete_release() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.1"]);
    complete(dir.path(), "v0.3.1");

    let output = stable_version_where(
        dir.path(),
        "api repos/{owner}/{repo}/contents/Cargo.toml?ref=stable",
        "gh: Not Found (HTTP 404)",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "version=0.3.1\n");
}

#[test]
fn no_release_at_all_moves_nothing() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &[]);
    stable(dir.path(), "0.3.1");

    let output = stable_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_stable_branch_that_cannot_be_read_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.1"]);
    complete(dir.path(), "v0.3.1");

    let output = stable_version_where(
        dir.path(),
        "api repos/{owner}/{repo}/contents/Cargo.toml?ref=stable",
        "HTTP 502: Bad Gateway",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "the stable branch could not be read: HTTP 502: Bad Gateway\n"
    );
}

#[test]
fn a_stable_branch_naming_no_version_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    fs::write(
        dir.path().join("replies/Cargo.toml?ref=stable"),
        "[package]\nname = \"conveyor\"\n",
    )
    .unwrap();

    let output = stable_version(dir.path());

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Cargo.toml on the stable branch names no version; point stable at a release tag by hand\n"
    );
}

#[test]
fn a_release_list_that_cannot_be_read_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    stable(dir.path(), "0.3.1");

    let output = stable_version_where(dir.path(), "release list", "HTTP 502: Bad Gateway");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
}

fn stable_version(root: &Path) -> Output {
    stable_version_where(root, "no call fails", "")
}

fn stable_version_where(root: &Path, failing_call: &str, failure: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(which("bash")))
        .arg(root.join("work/scripts/stable-version.sh"))
        .current_dir(root.join("work"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .env("REPLIES", root.join("replies"))
        .env("FAILING_CALL", failing_call)
        .env("FAILING_STDERR", failure)
        .output()
        .unwrap()
}

fn releases(root: &Path, tags: &[&str]) {
    let listing: String = tags.iter().map(|tag| format!("{tag}\n")).collect();
    fs::write(root.join("replies/release list"), listing).unwrap();
}

fn complete(root: &Path, tag: &str) {
    fs::write(root.join("replies").join(tag), format!("false\n{ASSETS}")).unwrap();
}

fn short(root: &Path, tag: &str) {
    let short = ASSETS.replace("conveyor-x86_64-apple-darwin.tar.gz\n", "");
    fs::write(root.join("replies").join(tag), format!("false\n{short}")).unwrap();
}

fn stable(root: &Path, version: &str) {
    fs::write(
        root.join("replies/Cargo.toml?ref=stable"),
        format!("[package]\nname = \"conveyor\"\nversion = \"{version}\"\nedition = \"2024\"\n"),
    )
    .unwrap();
}

fn sandbox(root: &Path) {
    let work = root.join("work");
    link_script(&work, "stable-version.sh");
    link_script(&work, "releases.sh");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("replies")).unwrap();
    symlink(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reply-call.sh"),
        root.join("bin/gh"),
    )
    .unwrap();
}

const ASSETS: &str = "conveyor-aarch64-apple-darwin.tar.gz\nconveyor-aarch64-apple-darwin.tar.gz.sha256\nconveyor-x86_64-apple-darwin.tar.gz\nconveyor-x86_64-apple-darwin.tar.gz.sha256\nconveyor-x86_64-unknown-linux-gnu.tar.gz\nconveyor-x86_64-unknown-linux-gnu.tar.gz.sha256\n";
