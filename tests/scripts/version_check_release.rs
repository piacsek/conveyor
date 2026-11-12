use std::fs;
use std::path::Path;

use crate::helpers::stderr;
use crate::version_check_cargo::checked_manifest;

#[test]
fn a_quoted_version_key_with_a_decoy_line_in_an_inline_table_is_refused() {
    let output = checked_manifest(
        "[package]\nname = \"conveyor\"\n\"version\" = \"0.0.0\"\nmetadata = { decoy = {\nversion = \"0.0.0\"\n} }\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), REFUSED);
}

#[test]
fn a_version_line_in_a_table_above_the_package_is_refused() {
    let output = checked_manifest(
        "[package.metadata.acme]\nversion = \"0.3.1\"\n\n[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), REFUSED);
}

#[test]
fn a_root_version_line_planted_above_the_package_is_refused_even_holding_the_old_fixed_probe() {
    let output = checked_manifest(
        "version = \"987.654.321\"\n\n[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), REFUSED);
}

#[test]
fn the_rehearsal_draws_its_probe_from_dev_urandom_on_every_run() {
    let check =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/version-check.sh"))
            .unwrap();

    assert!(check.contains("od -An -N6 -tu2 /dev/urandom"));
    assert!(!check.contains("987.654.321"));
}

#[test]
fn a_package_version_inherited_from_the_workspace_is_refused() {
    let output = checked_manifest(
        "[workspace]\n\n[workspace.package]\nversion = \"0.0.0\"\n\n[package]\nname = \"conveyor\"\nversion.workspace = true\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), REFUSED);
}

#[test]
fn a_manifest_in_the_plain_form_with_dependency_tables_passes() {
    let output = checked_manifest(
        "[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies.chrono]\nversion = \"0.4\"\n\n[package.metadata.acme]\nversion = \"0.3.1\"\n",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn the_check_and_the_release_commit_read_the_version_line_with_the_expression_the_release_build_uses()
 {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    let check = fs::read_to_string(root.join("scripts/version-check.sh")).unwrap();
    let commit = fs::read_to_string(root.join("scripts/release-commit.sh")).unwrap();

    assert!(release.contains(BUILD_READ));
    assert!(check.contains(BUILD_READ));
    assert!(commit.contains(BUILD_READ));
}

const BUILD_READ: &str = "grep -m1 '^version = ' Cargo.toml | cut -d'\"' -f2";

pub const REFUSED: &str = "version check: the release workflow could not set the version in Cargo.toml: rehearsed with a random probe version, cargo, the release build and the release script did not all read it back, so after the merge the release run would push its tag and then fail to build it; keep the plain form: `[package]` alone on its line, `version = \"0.0.0\"` as its own line under it, and no other `version = ` line above it\n";
