use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::helpers::{calls, git, isolated, link_script, repository, stderr, stdout};

#[test]
fn release_tags_main_pushes_only_that_tag_and_checks_what_the_run_published() {
    let dir = tempfile::tempdir().unwrap();
    let work = release_sandbox(dir.path());
    git(&work, &["tag", "v0.1.0"]);
    published(dir.path(), "0.24.0");

    let output = release(dir.path(), "0.24.0");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "gh run list --workflow release.yml --json status,databaseId --jq .[] | select(.status != \"completed\") | .databaseId",
            "gh run list --workflow release.yml --branch v0.24.0 --limit 1 --json databaseId --jq .[].databaseId",
            "gh run watch 77 --exit-status",
            "gh release view v0.24.0 --json assets --jq .assets[].name",
            "gh api repos/piacsek/homebrew-tap/contents/Formula/conveyor.rb -H Accept: application/vnd.github.raw+json",
            "gh release view v0.24.0 --json url --jq .url",
        ]
    );
    assert_eq!(origin_tags(dir.path()), "v0.24.0");
    assert_eq!(
        origin_commit(dir.path(), "v0.24.0"),
        origin_commit(dir.path(), "main")
    );
    assert_eq!(
        stdout(&output).lines().last(),
        Some("https://github.com/acme/conveyor/releases/tag/v0.24.0")
    );
}

#[test]
fn release_takes_the_version_with_its_leading_v() {
    let dir = tempfile::tempdir().unwrap();
    release_sandbox(dir.path());
    published(dir.path(), "0.24.0");

    let output = release(dir.path(), "v0.24.0");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(origin_tags(dir.path()), "v0.24.0");
}

#[test]
fn release_refuses_a_dirty_working_tree() {
    let dir = tempfile::tempdir().unwrap();
    let work = release_sandbox(dir.path());
    fs::write(work.join("notes.txt"), "untracked\n").unwrap();

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "the working tree is dirty; commit or stash everything before releasing\n"
    );
    assert_eq!(origin_tags(dir.path()), "");
}

#[test]
fn release_refuses_a_branch_that_is_not_main() {
    let dir = tempfile::tempdir().unwrap();
    let work = release_sandbox(dir.path());
    git(&work, &["checkout", "-qb", "topic-2"]);

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "on topic-2, not main; switch to main, the release is tagged from what was merged\n"
    );
    assert_eq!(origin_tags(dir.path()), "");
}

#[test]
fn release_refuses_a_main_with_commits_origin_does_not_have() {
    let dir = tempfile::tempdir().unwrap();
    let work = release_sandbox(dir.path());
    git(&work, &["commit", "-q", "--allow-empty", "-m", "local"]);

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), NOT_ORIGIN_MAIN);
    assert_eq!(origin_tags(dir.path()), "");
}

#[test]
fn release_fetches_before_comparing_so_a_main_behind_origin_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = release_sandbox(dir.path());
    git(&work, &["commit", "-q", "--allow-empty", "-m", "merged"]);
    git(&work, &["push", "-q", "origin", "main"]);
    git(&work, &["reset", "-q", "--hard", "HEAD~1"]);
    git(&work, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), NOT_ORIGIN_MAIN);
    assert_eq!(origin_tags(dir.path()), "");
}

#[test]
fn release_refuses_a_version_cargo_toml_does_not_carry() {
    let dir = tempfile::tempdir().unwrap();
    release_sandbox(dir.path());

    let output = release(dir.path(), "0.25.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Cargo.toml is at 0.24.0, not 0.25.0; bump it, add the CHANGELOG.md entry and merge that to main first\n"
    );
    assert_eq!(origin_tags(dir.path()), "");
}

#[test]
fn release_refuses_a_tag_that_exists_locally() {
    let dir = tempfile::tempdir().unwrap();
    let work = release_sandbox(dir.path());
    git(&work, &["tag", "v0.24.0"]);

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "v0.24.0 already exists locally; if it was never pushed, delete it with git tag -d v0.24.0\n"
    );
    assert_eq!(origin_tags(dir.path()), "");
}

#[test]
fn release_refuses_a_tag_that_exists_on_origin() {
    let dir = tempfile::tempdir().unwrap();
    let work = release_sandbox(dir.path());
    git(&work, &["tag", "v0.24.0"]);
    git(&work, &["push", "-q", "origin", "v0.24.0"]);
    git(&work, &["tag", "-d", "v0.24.0"]);

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "v0.24.0 already exists on origin; a released tag never moves, bump the version instead\n"
    );
    assert!(calls(dir.path()).is_empty());
}

#[test]
fn release_refuses_while_another_release_run_is_in_flight() {
    let dir = tempfile::tempdir().unwrap();
    release_sandbox(dir.path());
    published(dir.path(), "0.24.0");
    answer(dir.path(), "--json status", "76\n");

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "a release run is queued or in progress; wait for it (gh run list --workflow release.yml), then run this again\n"
    );
    assert_eq!(calls(dir.path()).len(), 1);
    assert_eq!(origin_tags(dir.path()), "");
}

#[test]
fn release_says_so_when_the_pushed_tag_starts_no_run() {
    let dir = tempfile::tempdir().unwrap();
    release_sandbox(dir.path());
    published(dir.path(), "0.24.0");
    answer(dir.path(), "--branch", "");

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "no release run appeared for v0.24.0; the tag is pushed, so do not run this again: look at gh run list --workflow release.yml\n"
    );
    assert_eq!(calls(dir.path()).len(), 3);
    assert_eq!(origin_tags(dir.path()), "v0.24.0");
}

#[test]
fn release_names_the_tarball_the_release_lacks() {
    let dir = tempfile::tempdir().unwrap();
    release_sandbox(dir.path());
    published(dir.path(), "0.24.0");
    let without_intel_mac = ASSETS.replace("conveyor-x86_64-apple-darwin.tar.gz\n", "");
    answer(dir.path(), "assets", &without_intel_mac);

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "release v0.24.0 lacks conveyor-x86_64-apple-darwin.tar.gz; rerun what failed with gh run rerun 77 --failed\n"
    );
}

#[test]
fn release_refuses_an_asset_nobody_expects() {
    let dir = tempfile::tempdir().unwrap();
    release_sandbox(dir.path());
    published(dir.path(), "0.24.0");
    answer(dir.path(), "assets", &format!("{ASSETS}conveyor.zip\n"));

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "release v0.24.0 holds more than the three tarballs and their .sha256; look at gh release view v0.24.0\n"
    );
}

#[test]
fn release_says_so_when_the_tap_formula_names_another_version() {
    let dir = tempfile::tempdir().unwrap();
    release_sandbox(dir.path());
    published(dir.path(), "0.24.0");
    answer(dir.path(), "homebrew-tap", &formula("0.23.2"));

    let output = release(dir.path(), "0.24.0");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "the tap formula does not name v0.24.0; run scripts/homebrew-formula.sh 0.24.0 > ~/projects/homebrew-tap/Formula/conveyor.rb and push the tap\n"
    );
}

const NOT_ORIGIN_MAIN: &str = "local main is not origin/main; run git pull --ff-only, and land local commits through a pull request\n";

const ASSETS: &str = "conveyor-aarch64-apple-darwin.tar.gz
conveyor-aarch64-apple-darwin.tar.gz.sha256
conveyor-x86_64-apple-darwin.tar.gz
conveyor-x86_64-apple-darwin.tar.gz.sha256
conveyor-x86_64-unknown-linux-gnu.tar.gz
conveyor-x86_64-unknown-linux-gnu.tar.gz.sha256
";

fn release_sandbox(root: &Path) -> PathBuf {
    let work = repository(root);
    git(root, &["init", "-q", "--bare", "origin.git"]);
    git(&work, &["remote", "add", "origin", "../origin.git"]);
    git(&work, &["checkout", "-q", "main"]);
    let manifest = "[package]\nname = \"conveyor\"\nversion = \"0.24.0\"\n";
    fs::write(work.join("Cargo.toml"), manifest).unwrap();
    link_script(&work, "release.sh");
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "Release v0.24.0"]);
    git(&work, &["push", "-q", "origin", "main"]);
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("replies")).unwrap();
    symlink(replier(), root.join("bin/gh")).unwrap();
    work
}

fn replier() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reply-call.sh")
}

fn answer(root: &Path, to_a_call_containing: &str, reply: &str) {
    fs::write(root.join("replies").join(to_a_call_containing), reply).unwrap();
}

fn published(root: &Path, version: &str) {
    answer(root, "--branch", "77\n");
    answer(root, "assets", ASSETS);
    answer(root, "homebrew-tap", &formula(version));
    let url = format!("https://github.com/acme/conveyor/releases/tag/v{version}\n");
    answer(root, "--json url", &url);
}

fn formula(version: &str) -> String {
    format!(
        "class Conveyor < Formula\n  url \"https://github.com/acme/conveyor/releases/download/v{version}/conveyor-aarch64-apple-darwin.tar.gz\"\nend\n"
    )
}

fn release(root: &Path, version: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(root.join("work/scripts/release.sh")))
        .arg(version)
        .env("HOME", root.join("home"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .env("REPLIES", root.join("replies"))
        .env("RELEASE_POLL_ATTEMPTS", "2")
        .env("RELEASE_POLL_INTERVAL", "0")
        .output()
        .unwrap()
}

fn origin_tags(root: &Path) -> String {
    in_origin(root, &["tag", "--list"])
}

fn origin_commit(root: &Path, revision: &str) -> String {
    in_origin(root, &["rev-parse", revision])
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
