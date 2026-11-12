use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{calls, isolated, link_script, stderr, stdout, which};

#[test]
fn a_formula_behind_the_newest_complete_release_is_brought_to_it() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    complete(dir.path(), "v0.3.2");
    formula(dir.path(), "0.3.1");

    let output = formula_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "version=0.3.2\n");
}

#[test]
fn a_formula_naming_the_newest_complete_release_is_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    complete(dir.path(), "v0.3.2");
    formula(dir.path(), "0.3.2");

    let output = formula_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_formula_ahead_of_every_complete_release_never_goes_back() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    short(dir.path(), "v0.3.2");
    formula(dir.path(), "0.3.2");

    let output = formula_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_newest_release_short_of_an_asset_is_passed_over_for_the_one_below() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1", "v0.3.0"]);
    complete(dir.path(), "v0.3.0");
    complete(dir.path(), "v0.3.1");
    short(dir.path(), "v0.3.2");
    formula(dir.path(), "0.3.0");

    let output = formula_version(dir.path());

    assert_eq!(stdout(&output), "version=0.3.1\n");
}

#[test]
fn a_newest_release_still_a_draft_is_passed_over() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    fs::write(dir.path().join("replies/v0.3.2"), format!("true\n{ASSETS}")).unwrap();
    formula(dir.path(), "0.3.0");

    let output = formula_version(dir.path());

    assert_eq!(stdout(&output), "version=0.3.1\n");
}

#[test]
fn versions_are_compared_as_numbers_not_as_text() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.9.0", "v0.10.0"]);
    complete(dir.path(), "v0.9.0");
    complete(dir.path(), "v0.10.0");
    formula(dir.path(), "0.9.0");

    let output = formula_version(dir.path());

    assert_eq!(stdout(&output), "version=0.10.0\n");
}

#[test]
fn releases_at_or_below_the_formula_are_never_read() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.2", "v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    complete(dir.path(), "v0.3.2");
    formula(dir.path(), "0.3.2");

    formula_version(dir.path());

    assert!(
        !calls(dir.path()).iter().any(|call| call.contains("v0.3.1")),
        "{:?}",
        calls(dir.path())
    );
}

#[test]
fn no_release_at_all_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &[]);
    formula(dir.path(), "0.3.1");

    let output = formula_version(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_tap_without_the_formula_gets_the_newest_complete_release() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.1"]);
    complete(dir.path(), "v0.3.1");

    let output = formula_version_where(
        dir.path(),
        "api repos/piacsek/homebrew-tap",
        "gh: Not Found (HTTP 404)",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "version=0.3.1\n");
}

#[test]
fn a_formula_that_cannot_be_read_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.1"]);
    complete(dir.path(), "v0.3.1");

    let output = formula_version_where(
        dir.path(),
        "api repos/piacsek/homebrew-tap",
        "HTTP 502: Bad Gateway",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "the tap formula could not be read: HTTP 502: Bad Gateway\n"
    );
}

#[test]
fn a_formula_naming_no_version_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    releases(dir.path(), &["v0.3.1"]);
    complete(dir.path(), "v0.3.1");
    fs::write(
        dir.path().join("replies/conveyor.rb"),
        "class Conveyor < Formula\nend\n",
    )
    .unwrap();

    let output = formula_version(dir.path());

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Formula/conveyor.rb in piacsek/homebrew-tap names no release version; fix it by hand\n"
    );
}

#[test]
fn a_release_list_that_cannot_be_read_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    formula(dir.path(), "0.3.1");

    let output = formula_version_where(dir.path(), "release list", "HTTP 502: Bad Gateway");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
}

fn formula_version(root: &Path) -> Output {
    formula_version_where(root, "no call fails", "")
}

fn formula_version_where(root: &Path, failing_call: &str, failure: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(which("bash")))
        .arg(root.join("work/scripts/formula-version.sh"))
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

fn formula(root: &Path, version: &str) {
    fs::write(
        root.join("replies/conveyor.rb"),
        format!("class Conveyor < Formula\n  url \"https://github.com/piacsek/conveyor/releases/download/v{version}/conveyor-aarch64-apple-darwin.tar.gz\"\nend\n"),
    )
    .unwrap();
}

fn sandbox(root: &Path) {
    let work = root.join("work");
    link_script(&work, "formula-version.sh");
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
