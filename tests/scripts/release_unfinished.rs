use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{isolated, link_script, stderr, stdout, which};

#[test]
fn a_tag_without_a_release_is_unfinished() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());

    let output = unfinished_where(dir.path(), "release view v0.3.1", "release not found");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "unfinished=true\n");
}

#[test]
fn a_draft_holding_every_asset_is_unfinished() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    reply(dir.path(), &format!("true\n{ASSETS}"));

    let output = unfinished(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "unfinished=true\n");
}

#[test]
fn a_published_release_short_of_an_asset_is_unfinished() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    let short = ASSETS.replace("conveyor-x86_64-unknown-linux-gnu.tar.gz\n", "");
    reply(dir.path(), &format!("false\n{short}"));

    let output = unfinished(dir.path());

    assert_eq!(stdout(&output), "unfinished=true\n");
}

#[test]
fn a_published_release_holding_every_asset_is_finished() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    reply(dir.path(), &format!("false\n{ASSETS}"));

    let output = unfinished(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "unfinished=false\n");
}

#[test]
fn a_release_that_cannot_be_read_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());

    let output = unfinished_where(dir.path(), "release view v0.3.1", "HTTP 502: Bad Gateway");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "the release of v0.3.1 could not be read: HTTP 502: Bad Gateway\n"
    );
}

#[test]
fn a_release_holding_an_asset_nobody_expects_stops_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    reply(dir.path(), &format!("false\n{ASSETS}notes.txt\n"));

    let output = unfinished(dir.path());

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "the release of v0.3.1 holds notes.txt besides the three tarballs and their .sha256; remove it by hand\n"
    );
}

fn unfinished(root: &Path) -> Output {
    unfinished_where(root, "no call fails", "")
}

fn unfinished_where(root: &Path, failing_call: &str, failure: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(which("bash")))
        .arg(root.join("work/scripts/release-unfinished.sh"))
        .arg("v0.3.1")
        .current_dir(root.join("work"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .env("REPLIES", root.join("replies"))
        .env("FAILING_CALL", failing_call)
        .env("FAILING_STDERR", failure)
        .output()
        .unwrap()
}

fn reply(root: &Path, text: &str) {
    fs::write(root.join("replies/v0.3.1"), text).unwrap();
}

fn sandbox(root: &Path) {
    let work = root.join("work");
    link_script(&work, "release-unfinished.sh");
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
