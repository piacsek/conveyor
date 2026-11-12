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
