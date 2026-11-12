use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{committed, fragment, isolated, stderr, which};
use crate::release_commit::{git_stdout, release_commit, sandbox};

#[test]
fn a_version_cargo_does_not_read_after_the_rewrite_is_refused_leaving_the_tree_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(&work, "Cargo.toml", DECOY);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Cargo.toml: cargo does not read the package version 0.3.2 after the rewrite\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn without_cargo_on_path_the_release_is_written_from_the_rewrite_alone() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(&work, "Cargo.toml", DECOY);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit_without_cargo(&work, &["0.3.2"]);

    assert!(!Path::new("/usr/bin/cargo").exists());
    assert!(!Path::new("/bin/cargo").exists());
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        git_stdout(&work, &["status", "--porcelain"]),
        " M CHANGELOG.md\n M Cargo.lock\n M Cargo.toml\n"
    );
}

#[test]
fn a_version_the_release_build_does_not_read_after_the_rewrite_is_refused_leaving_the_tree_as_it_was()
 {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(&work, "Cargo.toml", PLANTED);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit_without_cargo(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Cargo.toml: the release build reads the version 0.3.1 after the rewrite, not 0.3.2, so it would refuse the tag\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

fn release_commit_without_cargo(work: &Path, args: &[&str]) -> Output {
    isolated(
        Command::new(which("bash"))
            .env("PATH", "/usr/bin:/bin")
            .arg(work.join("scripts/release-commit.sh"))
            .args(args),
    )
    .output()
    .unwrap()
}

const PLANTED: &str = "version = \"0.3.1\"\n\n[package]\nname = \"conveyor\"\nversion = \"0.3.1\"\nedition = \"2024\"\n";

const DECOY: &str = "[package]\nname = \"conveyor\"\n\"version\" = \"0.3.1\"\nedition = \"2024\"\nmetadata = { decoy = {\nversion = \"0.3.1\"\n} }\n";
