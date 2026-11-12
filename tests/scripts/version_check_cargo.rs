use std::fs;
use std::path::Path;
use std::process::Output;

use crate::helpers::{committed, fragment, stderr, stdout};
use crate::version_check::{main_with_a_release, version_check};

#[test]
fn a_package_version_other_than_0_0_0_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion = \"0.4.0\"\n\n[dependencies]\nserde = { version = \"1\" }\n",
    );
    fragment(&work, "topic", "kind: feat\n\n- A new key.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml sets the package version to 0.4.0: put back `version = \"0.0.0\"`, only the release workflow picks a version (from the kinds in changes/)\n"
    );
}

#[test]
fn a_lock_entry_other_than_0_0_0_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.3.2\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock sets the conveyor entry to 0.3.2: put back `version = \"0.0.0\"` under `name = \"conveyor\"`, only the release workflow picks a version (from the kinds in changes/)\n"
    );
}

#[test]
fn a_manifest_with_a_multi_line_string_is_refused_since_it_could_hide_the_version() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.toml",
        "[workspace]\nmetadata = '''\n[package]\nversion = \"0.0.0\"\n'''\n\n[package]\nname = \"conveyor\"\nversion = \"9.9.9\"\n",
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml holds a multi-line string (\"\"\" or '''): the check reads the version line by line, so write every string in Cargo.toml on one line\n"
    );
}

#[test]
fn a_package_table_with_two_version_lines_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\n version=\"9.9.9\"\n",
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml holds more than one `version` line in [package]: the check reads the version line by line, so keep the one `version = \"0.0.0\"`\n"
    );
}

#[test]
fn a_lock_with_two_conveyor_entries_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.0.0\"\n\n[[package]]\nname = \"conveyor\"\nversion = \"9.9.9\"\n",
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock holds more than one package named `conveyor`: the check reads the version line by line, so keep the one entry with `version = \"0.0.0\"`\n"
    );
}

#[test]
fn a_lock_with_a_multi_line_string_is_refused_since_cargo_never_writes_one() {
    let output = checked_lock(
        "version = 4\n\n[[package]]\nname = \"conveyor\"\nversion = \"9.9.9\"\n\n[metadata]\nnote = \"\"\"\n[[package]]\nname = \"conveyor\"\nversion = \"0.0.0\"\n\"\"\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock holds a multi-line string (\"\"\" or '''): cargo writes Cargo.lock in one shape, so regenerate it with cargo (`cargo update --workspace`) instead of editing it by hand\n"
    );
}

#[test]
fn a_lock_name_without_spaces_is_refused_since_cargo_never_writes_one() {
    let output = checked_lock(
        "version = 4\n\n[[package]]\nname=\"conveyor\"\nversion = \"9.9.9\"\n\n[metadata]\nname = \"conveyor\"\nversion = \"0.0.0\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock line 4 is a `name` or quoted key other than `name = \"<crate>\"` (letters, digits, `_` and `-`): cargo writes Cargo.lock in one shape, so regenerate it with cargo (`cargo update --workspace`) instead of editing it by hand\n"
    );
}

#[test]
fn a_lock_name_in_single_quotes_is_refused_since_cargo_never_writes_one() {
    let output = checked_lock(
        "version = 4\n\n[[package]]\nname = 'conveyor'\nversion = \"9.9.9\"\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.0.0\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock line 4 is a `name` or quoted key other than `name = \"<crate>\"` (letters, digits, `_` and `-`): cargo writes Cargo.lock in one shape, so regenerate it with cargo (`cargo update --workspace`) instead of editing it by hand\n"
    );
}

#[test]
fn a_lock_name_spelled_with_an_escape_in_a_quoted_key_is_refused() {
    let output = checked_lock(
        "version = 4\n\n[[package]]\n\"n\\u0061me\" = \"conveyor\"\nversion = \"9.9.9\"\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.0.0\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock line 4 is a `name` or quoted key other than `name = \"<crate>\"` (letters, digits, `_` and `-`): cargo writes Cargo.lock in one shape, so regenerate it with cargo (`cargo update --workspace`) instead of editing it by hand\n"
    );
}

#[test]
fn a_lock_entry_under_a_spaced_header_is_refused_since_cargo_never_writes_one() {
    let output = checked_lock(
        "version = 4\n\n[[ package ]]\nname = \"conveyor\"\nversion = \"9.9.9\"\n\n[metadata]\nname = \"conveyor\"\nversion = \"0.0.0\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock line 4 is a `name = \"conveyor\"` that is not right after a `[[package]]` line and right before its `version = \"...\"` line: cargo writes Cargo.lock in one shape, so regenerate it with cargo (`cargo update --workspace`) instead of editing it by hand\n"
    );
}

#[test]
fn a_lock_entry_whose_version_is_not_the_next_line_is_refused() {
    let output = checked_lock(
        "version = 4\n\n[[package]]\nname = \"conveyor\"\nsource = \"path\"\nversion = \"0.0.0\"\n",
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock line 4 is a `name = \"conveyor\"` that is not right after a `[[package]]` line and right before its `version = \"...\"` line: cargo writes Cargo.lock in one shape, so regenerate it with cargo (`cargo update --workspace`) instead of editing it by hand\n"
    );
}

#[test]
fn the_lock_of_this_repository_passes() {
    let lock =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")).unwrap();

    let output = checked_lock(&lock);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_lock_cargo_wrote_after_a_dependency_bump_passes() {
    let output = checked_lock(
        "# This file is automatically @generated by Cargo.\n\
         # It is not intended for manual editing.\n\
         version = 4\n\
         \n\
         [[package]]\n\
         name = \"conveyor\"\n\
         version = \"0.0.0\"\n\
         dependencies = [\n \"conveyor-macros\",\n \"serde\",\n]\n\
         \n\
         [[package]]\n\
         name = \"conveyor-macros\"\n\
         version = \"0.2.1\"\n\
         source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
         checksum = \"c982642fa9e8606056828ee9a8505737230110bb1099153c79efe865c59d12ba\"\n\
         \n\
         [[package]]\n\
         name = \"serde\"\n\
         version = \"1.0.229\"\n\
         source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
         checksum = \"683d7910e743518b0e34f1186f92494becacb047c7b6bf616c96772180fef923\"\n\
         dependencies = [\n \"serde_derive\",\n]\n\
         \n\
         [[package]]\n\
         name = \"serde_derive\"\n\
         version = \"1.0.229\"\n\
         source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
         checksum = \"8c1f2a7d5e3b4c6a9f0e1d2c3b4a5f6e7d8c9b0a1b2c3d4e5f6a7b8c9d0e1f2a\"\n",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_package_version_written_without_spaces_is_refused_as_unread() {
    let output = checked_manifest("[package]\nname = \"conveyor\"\nversion=\"0.0.0\"\n");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml has no `version = \"...\"` line under a plain `[package]` header: the check reads the version line by line, so write the header as `[package]` alone on its line and the version as `version = \"0.0.0\"`\n"
    );
}

#[test]
fn a_package_header_with_a_comment_is_refused_as_unread() {
    let output =
        checked_manifest("[package] # the crate\nname = \"conveyor\"\nversion = \"0.0.0\"\n");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml has no `version = \"...\"` line under a plain `[package]` header: the check reads the version line by line, so write the header as `[package]` alone on its line and the version as `version = \"0.0.0\"`\n"
    );
}

#[test]
fn a_manifest_with_crlf_line_endings_is_refused_as_unread() {
    let output = checked_manifest("[package]\r\nname = \"conveyor\"\r\nversion = \"0.0.0\"\r\n");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml has no `version = \"...\"` line under a plain `[package]` header: the check reads the version line by line, so write the header as `[package]` alone on its line and the version as `version = \"0.0.0\"`\n"
    );
}

#[test]
fn a_lock_without_a_conveyor_entry_is_refused_as_unread() {
    let output =
        checked_lock("version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.lock has no `name = \"conveyor\"` line with a `version = \"...\"` line right after it: cargo writes Cargo.lock in one shape, so regenerate it with cargo (`cargo update --workspace`) instead of editing it by hand\n"
    );
}

fn checked_lock(lock: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(&work, "Cargo.lock", lock);
    fragment(&work, "topic", "kind: internal\n");
    version_check(&work)
}

fn checked_manifest(manifest: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(&work, "Cargo.toml", manifest);
    fragment(&work, "topic", "kind: internal\n");
    version_check(&work)
}
