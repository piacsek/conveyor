use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{committed, fragment, git, isolated, recorder, stderr, stdout, which};
use crate::version_check::{main_with_a_release, version_check};

#[test]
fn a_decoy_version_line_in_a_multi_line_inline_table_does_not_hide_the_version_cargo_reads() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nmetadata = {\nversion = \"0.0.0\"\n}\n\"vers\\u0069on\" = \"1.2.3\"\n",
    );
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml sets the package version to 1.2.3: put back `version = \"0.0.0\"`, only the release workflow picks a version (from the kinds in changes/)\n"
    );
}

#[test]
fn a_build_script_of_the_head_never_runs() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    let marker = dir.path().join("built");
    committed(
        &work,
        "build.rs",
        &format!("fn main() {{ std::fs::write({marker:?}, \"\").unwrap(); }}\n"),
    );
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion=\"0.0.0\"\nbuild = \"build.rs\"\n",
    );
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert!(!marker.exists());
}

#[test]
fn a_cargo_config_of_the_head_never_runs_a_wrapper_an_alias_or_sets_an_environment() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    let calls = dir.path().join("calls.log");
    let program = recorder();
    committed(
        &work,
        ".cargo/config.toml",
        &format!(
            "[build]\nrustc-wrapper = {program:?}\nrustc = {program:?}\n\n[alias]\nmetadata = [\"--version\"]\nm = [\"--version\"]\n\n[env]\nCALLS_LOG = {calls:?}\n"
        ),
    );
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion=\"0.0.0\"\n",
    );
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check_with_wrapper_env(&work, &calls);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert!(!calls.exists());
}

#[test]
fn a_toolchain_file_of_the_head_naming_no_installed_toolchain_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "rust-toolchain.toml",
        "[toolchain]\nchannel = \"nonexistent-toolchain\"\n",
    );
    committed(&work, "src/rust-toolchain", "nonexistent-toolchain\n");
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion=\"0.0.0\"\n",
    );
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_manifest_cargo_cannot_load_is_refused_with_cargos_first_error_line() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\n version=\"9.9.9\"\n",
    );
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml does not load with cargo, which answers `error: duplicate key`: the manifest must load with cargo (cargo metadata), fix it there\n"
    );
}

#[test]
fn a_virtual_manifest_is_refused_since_conveyor_must_be_the_root_package() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "app/Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\n",
    );
    committed(&work, "app/src/main.rs", "fn main() {}\n");
    committed(&work, "Cargo.toml", "[workspace]\nmembers = [\"app\"]\n");
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml has no package of its own (cargo reads a virtual manifest): the package conveyor belongs at the root, where the release workflow writes its version\n"
    );
}

#[test]
fn a_root_package_named_otherwise_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyance\"\nversion = \"0.0.0\"\n",
    );
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "version check: Cargo.toml names its package conveyance: the package at the root must be conveyor, the one the release workflow versions\n"
    );
}

#[test]
fn a_workspace_member_keeps_its_own_version() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    committed(
        &work,
        "helper/Cargo.toml",
        "[package]\nname = \"helper\"\nversion = \"1.4.0\"\n",
    );
    committed(&work, "helper/src/lib.rs", "\n");
    committed(
        &work,
        "Cargo.toml",
        "[package]\nname = \"conveyor\"\nversion = \"0.0.0\"\n\n[workspace]\nmembers = [\"helper\"]\n",
    );
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_machine_without_cargo_fails_the_check_instead_of_passing_it() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    fragment(&work, "topic", "kind: internal\n");
    git(&work, &["checkout", "-q", "main"]);

    let output = isolated(
        Command::new(which("bash"))
            .current_dir(&work)
            .env("PATH", "/usr/bin:/bin")
            .arg(work.join("scripts/version-check.sh"))
            .args(["main", "pull"]),
    )
    .output()
    .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "version check: cargo is not on PATH: the check asks cargo for the version in Cargo.toml, so install Rust and run it again\n"
    );
}

#[test]
fn the_manifest_of_this_repository_passes() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    let manifest =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    committed(&work, "Cargo.toml", &manifest);
    fragment(&work, "topic", "kind: internal\n");

    let output = version_check(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn the_head_is_written_to_a_temporary_directory_that_is_gone_afterwards() {
    let dir = tempfile::tempdir().unwrap();
    let work = main_with_a_release(dir.path());
    let temporary = dir.path().join("temporary");
    fs::create_dir_all(&temporary).unwrap();
    fragment(&work, "topic", "kind: internal\n");
    git(&work, &["checkout", "-q", "main"]);

    let output = isolated(
        Command::new(which("bash"))
            .current_dir(&work)
            .env("TMPDIR", &temporary)
            .arg(work.join("scripts/version-check.sh"))
            .args(["main", "pull"]),
    )
    .output()
    .unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(fs::read_dir(&temporary).unwrap().count(), 0);
}

fn version_check_with_wrapper_env(work: &Path, calls: &Path) -> Output {
    git(work, &["checkout", "-q", "main"]);
    isolated(
        Command::new(which("bash"))
            .current_dir(work)
            .env("RUSTC_WRAPPER", recorder())
            .env("RUSTC_WORKSPACE_WRAPPER", recorder())
            .env("CARGO_BUILD_RUSTC", recorder())
            .env("CALLS_LOG", calls)
            .arg(work.join("scripts/version-check.sh"))
            .args(["main", "pull"]),
    )
    .output()
    .unwrap()
}
