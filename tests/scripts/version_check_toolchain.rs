use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::helpers::{committed, fragment, git, isolated, stderr, stdout, which};
use crate::version_check::{main_with_a_release, version_check};

const LOCKED: &str = "1111111111111111111111111111111111111111";
const UPDATED: &str = "2222222222222222222222222222222222222222";
const PASSED: &str = "the pull request leaves the version to the release workflow\n";
const UPDATE: &str = "without Nix: `docker run --rm -v \"$PWD\":/src -w /src nixos/nix sh -c \"git config --global --add safe.directory /src; nix --extra-experimental-features 'nix-command flakes' flake update rust-overlay\"`";

#[test]
fn a_new_channel_with_the_same_overlay_revision_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "rust-toolchain.toml", &toolchain("1.99.0"));

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), kept());
}

#[test]
fn a_new_channel_with_a_new_overlay_revision_passes() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "rust-toolchain.toml", &toolchain("1.99.0"));
    committed(&work, "flake.lock", &lock(UPDATED));

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), PASSED);
}

#[test]
fn the_same_channel_with_the_same_lock_passes() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(
        &work,
        "rust-toolchain.toml",
        "[toolchain]\nchannel = \"1.98.1\"\ncomponents = [\"rustfmt\"]\n",
    );

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), PASSED);
}

#[test]
fn the_same_channel_with_a_new_lock_passes() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "flake.lock", &lock(UPDATED));

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), PASSED);
}

#[test]
fn the_same_channel_without_a_lock_passes() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    git(&work, &["rm", "-q", "flake.lock"]);
    git(&work, &["commit", "-qm", "drop the flake"]);

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), PASSED);
}

#[test]
fn a_new_channel_without_a_lock_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "rust-toolchain.toml", &toolchain("1.99.0"));
    git(&work, &["rm", "-q", "flake.lock"]);
    git(&work, &["commit", "-qm", "drop the flake"]);

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), missing());
}

#[test]
fn a_new_channel_with_a_lock_that_has_no_overlay_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "rust-toolchain.toml", &toolchain("1.99.0"));
    committed(
        &work,
        "flake.lock",
        "{\"nodes\":{\"root\":{\"inputs\":{\"nixpkgs\":\"nixpkgs\"}},\"nixpkgs\":{\"locked\":{\"rev\":\"2222222222222222222222222222222222222222\"}}},\"root\":\"root\",\"version\":7}\n",
    );

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), missing());
}

#[test]
fn a_new_channel_with_a_lock_that_is_not_json_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "rust-toolchain.toml", &toolchain("1.99.0"));
    committed(
        &work,
        "flake.lock",
        "rev = \"2222222222222222222222222222222222222222\"\n",
    );

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), missing());
}

#[test]
fn a_new_channel_with_an_overlay_revision_that_is_not_a_commit_is_refused_and_never_printed() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "rust-toolchain.toml", &toolchain("1.99.0"));
    committed(&work, "flake.lock", &lock("::warning::spoof"));

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), missing());
}

#[test]
fn a_head_without_the_toolchain_file_counts_as_a_new_channel() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    git(&work, &["rm", "-q", "rust-toolchain.toml"]);
    git(&work, &["commit", "-qm", "drop the toolchain"]);

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), kept());
}

#[test]
fn a_machine_without_jq_refuses_a_new_channel_instead_of_passing_it() {
    let dir = tempfile::tempdir().unwrap();
    let work = pinned(dir.path());
    committed(&work, "rust-toolchain.toml", &toolchain("1.99.0"));
    committed(&work, "flake.lock", &lock(UPDATED));
    git(&work, &["checkout", "-q", "main"]);
    let bin = without_jq(dir.path());

    let output = isolated(
        Command::new(which("bash"))
            .current_dir(&work)
            .env("PATH", &bin)
            .arg(work.join("scripts/version-check.sh"))
            .args(["main", "pull"]),
    )
    .output()
    .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: jq is not on PATH: the check reads the locked rust-overlay revision in flake.lock with jq when rust-toolchain.toml changes the Rust channel, so install jq and run it again\n"
    );
}

fn pinned(root: &Path) -> PathBuf {
    let work = main_with_a_release(root);
    git(&work, &["checkout", "-q", "main"]);
    committed(&work, "rust-toolchain.toml", &toolchain("1.98.1"));
    committed(&work, "flake.lock", &lock(LOCKED));
    git(&work, &["checkout", "-qB", "pull"]);
    fragment(&work, "topic", "kind: internal\n");
    work
}

fn without_jq(root: &Path) -> PathBuf {
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let path = std::env::var_os("PATH").unwrap();
    for dir in std::env::split_paths(&path) {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name != "jq" && fs::symlink_metadata(bin.join(&name)).is_err() {
                std::os::unix::fs::symlink(entry.path(), bin.join(&name)).unwrap();
            }
        }
    }
    bin
}

fn toolchain(channel: &str) -> String {
    format!("[toolchain]\nchannel = \"{channel}\"\ncomponents = [\"rustfmt\", \"clippy\"]\n")
}

fn lock(rev: &str) -> String {
    format!(
        "{{\"nodes\":{{\"nixpkgs\":{{\"locked\":{{\"rev\":\"{LOCKED}\"}}}},\"root\":{{\"inputs\":{{\"nixpkgs\":\"nixpkgs\",\"rust-overlay\":\"rust-overlay_2\"}}}},\"rust-overlay_2\":{{\"inputs\":{{\"nixpkgs\":[\"nixpkgs\"]}},\"locked\":{{\"rev\":\"{rev}\",\"type\":\"github\"}}}}}},\"root\":\"root\",\"version\":7}}\n"
    )
}

fn kept() -> String {
    format!(
        "version check: rust-toolchain.toml changes the Rust channel and flake.lock keeps the locked rust-overlay revision: the Nix flake builds that channel through rust-overlay, which knows no Rust released after its locked revision, so update it in this pull request ({UPDATE})\n"
    )
}

fn missing() -> String {
    format!(
        "version check: rust-toolchain.toml changes the Rust channel and flake.lock holds no locked rust-overlay revision (40 hex digits under the node the root's `rust-overlay` input names): the Nix flake builds that channel through rust-overlay, so lock it in this pull request ({UPDATE})\n"
    )
}
