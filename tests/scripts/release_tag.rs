use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{fragment, git, isolated, link_script, repository, stderr, stdout, which};

#[test]
fn a_finished_release_with_nothing_pending_prints_nothing_and_tags_nothing() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    finished(dir.path(), "v0.3.1");

    let output = release_tag(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(origin_tags(dir.path()), "v0.3.1");
}

#[test]
fn a_pending_fix_is_tagged_as_the_next_patch_and_only_that_tag_is_pushed() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    finished(dir.path(), "v0.3.1");
    merged(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    let main = in_origin(dir.path(), &["rev-parse", "main"]);

    let output = release_tag(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "version=0.3.2\ntag=v0.3.2\n");
    assert_eq!(origin_tags(dir.path()), "v0.3.1\nv0.3.2");
    assert_eq!(in_origin(dir.path(), &["rev-parse", "main"]), main);
}

#[test]
fn the_release_tag_is_a_commit_on_top_of_main_that_carries_the_version() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    finished(dir.path(), "v0.3.1");
    merged(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    let main = in_origin(dir.path(), &["rev-parse", "main"]);

    release_tag(dir.path());

    assert_eq!(in_origin(dir.path(), &["rev-parse", "v0.3.2^"]), main);
    assert_eq!(
        in_origin(
            dir.path(),
            &["log", "-1", "--format=%an <%ae> %s", "v0.3.2"]
        ),
        "github-actions[bot] <github-actions[bot]@users.noreply.github.com> Release v0.3.2"
    );
    assert_eq!(
        in_origin(dir.path(), &["show", "v0.3.2:Cargo.toml"]),
        "[package]\nname = \"conveyor\"\nversion = \"0.3.2\"\nedition = \"2024\""
    );
    assert!(
        in_origin(dir.path(), &["show", "v0.3.2:CHANGELOG.md"])
            .contains("- Fixed the queue.\n\n## v0.3.1 (2026-11-02)"),
        "{}",
        in_origin(dir.path(), &["show", "v0.3.2:CHANGELOG.md"])
    );
}

#[test]
fn a_last_tag_without_a_release_is_resumed_before_anything_new_is_tagged() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    merged(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_tag_where(dir.path(), "release view v0.3.1", "release not found");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "version=0.3.1\ntag=v0.3.1\n");
    assert_eq!(origin_tags(dir.path()), "v0.3.1");
}

#[test]
fn a_release_that_cannot_be_read_stops_the_run_before_anything_is_tagged() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    merged(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_tag_where(dir.path(), "release view v0.3.1", "HTTP 502: Bad Gateway");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "the release of v0.3.1 could not be read: HTTP 502: Bad Gateway\n"
    );
    assert_eq!(origin_tags(dir.path()), "v0.3.1");
}

#[test]
fn a_last_release_short_of_an_asset_is_resumed() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    merged(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    let short = ASSETS.replace("conveyor-x86_64-apple-darwin.tar.gz.sha256\n", "");
    fs::write(dir.path().join("replies/v0.3.1"), short).unwrap();

    let output = release_tag(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "version=0.3.1\ntag=v0.3.1\n");
    assert_eq!(origin_tags(dir.path()), "v0.3.1");
}

#[test]
fn a_last_release_holding_an_asset_nobody_expects_stops_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    merged(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    let extra = format!("{ASSETS}notes.txt\n");
    fs::write(dir.path().join("replies/v0.3.1"), extra).unwrap();

    let output = release_tag(dir.path());

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "the release of v0.3.1 holds notes.txt besides the three tarballs and their .sha256; remove it by hand\n"
    );
    assert_eq!(origin_tags(dir.path()), "v0.3.1");
}

#[test]
fn outside_github_actions_nothing_is_tagged() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    finished(dir.path(), "v0.3.1");
    merged(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = command(dir.path())
        .env_remove("GITHUB_ACTIONS")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "scripts/release-tag.sh runs in the release workflow only; merge to main and watch the release run\n"
    );
    assert_eq!(origin_tags(dir.path()), "v0.3.1");
}

fn release_tag(root: &Path) -> Output {
    release_tag_where(root, "no call fails", "")
}

fn release_tag_where(root: &Path, failing_call: &str, failure: &str) -> Output {
    command(root)
        .env("GITHUB_ACTIONS", "true")
        .env("FAILING_CALL", failing_call)
        .env("FAILING_STDERR", failure)
        .output()
        .unwrap()
}

fn command(root: &Path) -> Command {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let mut command = Command::new(which("bash"));
    isolated(&mut command)
        .arg(root.join("work/scripts/release-tag.sh"))
        .current_dir(root.join("work"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .env("REPLIES", root.join("replies"));
    command
}

fn merged(work: &Path, name: &str, text: &str) {
    fragment(work, name, text);
    git(work, &["push", "-q", "origin", "main"]);
}

fn finished(root: &Path, tag: &str) {
    fs::write(root.join("replies").join(tag), ASSETS).unwrap();
}

fn origin_tags(root: &Path) -> String {
    in_origin(root, &["tag", "--list"])
}

fn in_origin(root: &Path, args: &[&str]) -> String {
    let output = isolated(
        Command::new("git")
            .current_dir(root.join("origin.git"))
            .args(args),
    )
    .output()
    .unwrap();
    stdout(&output).trim().to_string()
}

fn sandbox(root: &Path) -> PathBuf {
    let work = repository(root);
    git(root, &["init", "-q", "--bare", "-b", "main", "origin.git"]);
    git(&work, &["remote", "add", "origin", "../origin.git"]);
    git(&work, &["checkout", "-q", "main"]);
    link_script(&work, "fragments.sh");
    link_script(&work, "next-version.sh");
    link_script(&work, "release-commit.sh");
    link_script(&work, "release-tag.sh");
    fs::write(work.join("Cargo.toml"), CARGO_TOML).unwrap();
    fs::write(work.join("Cargo.lock"), CARGO_LOCK).unwrap();
    fs::write(work.join("CHANGELOG.md"), CHANGELOG).unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "Release v0.3.1"]);
    git(&work, &["tag", "v0.3.1"]);
    git(&work, &["push", "-q", "origin", "main", "v0.3.1"]);
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("replies")).unwrap();
    symlink(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reply-call.sh"),
        root.join("bin/gh"),
    )
    .unwrap();
    work
}

const ASSETS: &str = "conveyor-aarch64-apple-darwin.tar.gz\nconveyor-aarch64-apple-darwin.tar.gz.sha256\nconveyor-x86_64-apple-darwin.tar.gz\nconveyor-x86_64-apple-darwin.tar.gz.sha256\nconveyor-x86_64-unknown-linux-gnu.tar.gz\nconveyor-x86_64-unknown-linux-gnu.tar.gz.sha256\n";

const CARGO_TOML: &str =
    "[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\nedition = \"2024\"\n";

const CARGO_LOCK: &str = "version = 4\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.0.0\"\n";

const CHANGELOG: &str = "# Changelog\n\n## v0.3.1 (2026-11-02)\n- Fixed the builds.\n";
