use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use conveyor::changelog::check;

use crate::helpers::{
    committed, fragment, git, isolated, link_script, repository, stderr, stdout, which,
};

#[test]
fn the_package_version_in_cargo_toml_becomes_the_release() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(work.join("Cargo.toml")).unwrap(),
        "[package]\nname = \"conveyor\"\nversion = \"0.3.2\"\nedition = \"2024\"\n\n[dependencies]\nchrono = { version = \"0.4\" }\n\n[package.metadata.acme]\nversion = \"0.3.1\"\n"
    );
}

#[test]
fn a_dependency_under_an_indented_table_header_keeps_its_version() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(
        &work,
        "Cargo.toml",
        &CARGO_TOML.replacen(
            "[dependencies]\nchrono = { version = \"0.4\" }\n",
            "  [dependencies.chrono]\nversion = \"0.4\"\n",
            1,
        ),
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(work.join("Cargo.toml")).unwrap(),
        "[package]\nname = \"conveyor\"\nversion = \"0.3.2\"\nedition = \"2024\"\n\n  [dependencies.chrono]\nversion = \"0.4\"\n\n[package.metadata.acme]\nversion = \"0.3.1\"\n"
    );
}

#[test]
fn only_the_conveyor_entry_of_cargo_lock_becomes_the_release() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(work.join("Cargo.lock")).unwrap(),
        "version = 4\n\n[[package]]\nname = \"chrono\"\nversion = \"0.3.1\"\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.3.2\"\ndependencies = [\n \"chrono\",\n]\n\n[[package]]\nname = \"conveyor-macros\"\nversion = \"0.3.1\"\n"
    );
}

#[test]
fn the_changelog_gains_the_pending_bullets_in_file_name_order_above_the_last_release() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "b-topic", "kind: fix\n\n- Fixed the queue.\n");
    fragment(
        &work,
        "a-topic",
        "kind: feat\n\n- A new key,\n  on every column.\n\n- A new default.\n",
    );

    let output = release_commit(&work, &["0.4.0"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
        format!(
            "# Changelog\n\n## v0.4.0 ({})\n- A new key,\n  on every column.\n- A new default.\n- Fixed the queue.\n\n## v0.3.1 (2026-11-02)\n- Fixed the builds.\n\n## v0.3.0 (2026-11-01)\n- A first key.\n",
            today()
        )
    );
}

#[test]
fn a_fragment_with_crlf_line_endings_reaches_the_changelog_without_carriage_returns() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(
        &work,
        "topic",
        "kind: fix\r\n\r\n- Fixed the queue,\r\n  on every column.\r\n",
    );

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
        format!(
            "# Changelog\n\n## v0.3.2 ({})\n- Fixed the queue,\n  on every column.\n\n## v0.3.1 (2026-11-02)\n- Fixed the builds.\n\n## v0.3.0 (2026-11-01)\n- A first key.\n",
            today()
        )
    );
}

#[test]
fn the_older_releases_come_from_the_last_tag_whatever_the_branch_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fs::write(
        work.join("CHANGELOG.md"),
        "# Changelog\n\n## v9.0.0 (2026-11-03)\n- Written by a pull request.\n",
    )
    .unwrap();
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
        format!(
            "# Changelog\n\n## v0.3.2 ({})\n- Fixed the queue.\n\n## v0.3.1 (2026-11-02)\n- Fixed the builds.\n\n## v0.3.0 (2026-11-01)\n- A first key.\n",
            today()
        )
    );
}

#[test]
fn a_release_without_a_bullet_says_internal_changes_only() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "a-topic", "kind: internal\n");
    fragment(&work, "b-topic", "kind: internal\n\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
        format!(
            "# Changelog\n\n## v0.3.2 ({})\n- v0.3.2: internal changes only.\n\n## v0.3.1 (2026-11-02)\n- Fixed the builds.\n\n## v0.3.0 (2026-11-01)\n- A first key.\n",
            today()
        )
    );
}

#[test]
fn the_changelog_it_writes_passes_the_rules_of_the_bundled_one() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(
        &work,
        "a-topic",
        "kind: feat\n\n- A new key,\n  on every column.\n",
    );
    fragment(&work, "b-topic", "kind: internal\n");

    let output = release_commit(&work, &["0.4.0"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        check(
            &fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
            "0.4.0"
        ),
        Ok(())
    );
}

#[test]
fn an_internal_only_changelog_passes_the_rules_of_the_bundled_one() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: internal\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        check(
            &fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
            "0.3.2"
        ),
        Ok(())
    );
}

#[test]
fn two_internal_only_releases_in_a_row_pass_the_rules_of_the_bundled_one() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "a-topic", "kind: internal\n");
    let first = release_commit(&work, &["0.3.2"]);
    assert_eq!(first.status.code(), Some(0), "{}", stderr(&first));
    git(&work, &["commit", "-qam", "Release v0.3.2"]);
    git(&work, &["tag", "v0.3.2"]);
    fragment(&work, "b-topic", "kind: internal\n");

    let output = release_commit(&work, &["0.3.3"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        check(
            &fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
            "0.3.3"
        ),
        Ok(())
    );
}

#[test]
fn the_release_is_left_in_the_working_tree_uncommitted() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    let head = git_stdout(&work, &["rev-parse", "HEAD"]);

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(git_stdout(&work, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git_stdout(&work, &["status", "--porcelain"]),
        " M CHANGELOG.md\n M Cargo.lock\n M Cargo.toml\n"
    );
}

#[test]
fn a_malformed_fragment_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "a-topic", "kind: fix\n\n- Fixed the queue.\n");
    fragment(&work, "b-topic", "kind: chore\n\n- Tidied up.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/b-topic.md must start with `kind: fix`, `kind: feat` or `kind: internal`\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_carriage_return_inside_a_bullet_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(
        &work,
        "topic",
        "kind: fix\n\n- Fixed the queue\r on every column.\n",
    );

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/topic.md holds a carriage return inside a line: a carriage return may only end a line, right before its newline\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_bullet_of_an_older_release_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "a-topic", "kind: fix\n\n- Fixed the queue.\n");
    fragment(&work, "b-topic", "kind: fix\n\n- A first key.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/b-topic.md repeats a bullet of v0.3.0: A first key.\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_bullet_of_an_older_release_wrapped_differently_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n-  A first\n\n\tkey. \n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/topic.md repeats a bullet of v0.3.0: A first key.\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_released_fragment_renamed_since_is_refused_by_its_new_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    let first = release_commit(&work, &["0.3.2"]);
    assert_eq!(first.status.code(), Some(0), "{}", stderr(&first));
    git(&work, &["commit", "-qam", "Release v0.3.2"]);
    git(&work, &["tag", "v0.3.2"]);
    git(&work, &["mv", "changes/topic.md", "changes/topic2.md"]);
    git(&work, &["commit", "-qm", "rename a released fragment"]);

    let output = release_commit(&work, &["0.3.3"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/topic2.md repeats a bullet of v0.3.2: Fixed the queue.\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn the_same_bullet_in_two_pending_fragments_passes_the_rules_of_the_bundled_one() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "a-topic", "kind: fix\n\n- Fixed the queue.\n");
    fragment(&work, "b-topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        check(
            &fs::read_to_string(work.join("CHANGELOG.md")).unwrap(),
            "0.3.2"
        ),
        Ok(())
    );
}

#[test]
fn a_paragraph_after_the_first_bullet_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(
        &work,
        "topic",
        "kind: feat\n\n- A new key.\n\nIt works on every column.\n",
    );

    let output = release_commit(&work, &["0.4.0"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/topic.md has a bullet, so every line after its kind must be a `- ` bullet, an indented continuation or blank: It works on every column.\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn prose_before_the_first_bullet_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(
        &work,
        "topic",
        "kind: fix\nIntro prose lost\n- Fixed the queue.\n",
    );

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/topic.md has a bullet, so every line after its kind must be a `- ` bullet, an indented continuation or blank: Intro prose lost\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn an_internal_bullet_followed_by_a_paragraph_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(
        &work,
        "topic",
        "kind: internal\n\n- Faster startup.\n\nThe stages load in parallel.\n",
    );

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/topic.md has a bullet, so every line after its kind must be a `- ` bullet, an indented continuation or blank: The stages load in parallel.\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_file_in_a_folder_under_changes_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    committed(&work, "changes/fix/sub.md", "kind: feat\n\n- A new key.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/fix/sub.md is not a fragment: a fragment is a file changes/<name>.md\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_released_fragment_edited_since_is_refused_by_name_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    git(&work, &["tag", "v0.3.2"]);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue again.\n");
    fragment(&work, "other", "kind: fix\n\n- Fixed the builds.\n");

    let output = release_commit(&work, &["0.3.3"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "changes/topic.md was released in v0.3.2 and has changed since; a released fragment never changes, so put the change in a new file\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_package_table_header_the_rewrite_misses_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(
        &work,
        "Cargo.toml",
        &CARGO_TOML.replacen("[package]\n", "[package] # c\n", 1),
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Cargo.toml: the version of [package] does not read 0.3.2 after the rewrite\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_package_version_line_the_rewrite_misses_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(
        &work,
        "Cargo.toml",
        &CARGO_TOML.replacen("version = \"0.3.1\"", "version=\"0.9.9\"", 1),
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Cargo.toml: the version of [package] does not read 0.3.2 after the rewrite\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_missed_package_version_is_refused_when_an_indented_table_reads_the_release() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(
        &work,
        "Cargo.toml",
        &CARGO_TOML
            .replacen("version = \"0.3.1\"", "version=\"0.9.9\"", 1)
            .replacen(
                "[dependencies]\nchrono = { version = \"0.4\" }\n",
                "  [dependencies.chrono]\nversion = \"0.3.2\"\n",
                1,
            ),
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Cargo.toml: the version of [package] does not read 0.3.2 after the rewrite\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_conveyor_lock_entry_the_rewrite_misses_is_refused_by_file() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    committed(
        &work,
        "Cargo.lock",
        &CARGO_LOCK.replace(
            "name = \"conveyor\"\nversion = \"0.3.1\"",
            "name = \"conveyor\"\nversion=\"0.3.1\"",
        ),
    );
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Cargo.lock: the version of the conveyor entry does not read 0.3.2 after the rewrite\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_release_with_nothing_pending_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());

    let output = release_commit(&work, &["0.3.2"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "no fragment in changes/ is pending since v0.3.1; nothing to release\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_version_that_is_not_three_numbers_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["v0.3"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "usage: scripts/release-commit.sh X.Y.Z\n");
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_version_with_a_leading_zero_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["01.2.3"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "usage: scripts/release-commit.sh X.Y.Z\n");
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn the_version_of_the_last_release_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.3.1"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "0.3.1 is not above the last release v0.3.1\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_version_below_the_last_release_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    git(&work, &["tag", "v0.9.10"]);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &["0.9.9"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "0.9.9 is not above the last release v0.9.10\n"
    );
    assert_eq!(git_stdout(&work, &["status", "--porcelain"]), "");
}

#[test]
fn a_missing_version_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = sandbox(dir.path());
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = release_commit(&work, &[]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "usage: scripts/release-commit.sh X.Y.Z\n");
}

fn release_commit(work: &Path, args: &[&str]) -> Output {
    isolated(
        Command::new(which("bash"))
            .arg(work.join("scripts/release-commit.sh"))
            .args(args),
    )
    .output()
    .unwrap()
}

fn git_stdout(work: &Path, args: &[&str]) -> String {
    let output = isolated(Command::new("git").current_dir(work).args(args))
        .output()
        .unwrap();
    stdout(&output)
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn sandbox(root: &Path) -> PathBuf {
    let work = repository(root);
    link_script(&work, "fragments.sh");
    link_script(&work, "release-commit.sh");
    fs::write(work.join("Cargo.toml"), CARGO_TOML).unwrap();
    fs::write(work.join("Cargo.lock"), CARGO_LOCK).unwrap();
    fs::write(work.join("CHANGELOG.md"), CHANGELOG).unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "Release v0.3.1"]);
    git(&work, &["tag", "v0.3.1"]);
    work
}

const CARGO_TOML: &str = "[package]\nname = \"conveyor\"\nversion = \"0.3.1\"\nedition = \"2024\"\n\n[dependencies]\nchrono = { version = \"0.4\" }\n\n[package.metadata.acme]\nversion = \"0.3.1\"\n";

const CARGO_LOCK: &str = "version = 4\n\n[[package]]\nname = \"chrono\"\nversion = \"0.3.1\"\n\n[[package]]\nname = \"conveyor\"\nversion = \"0.3.1\"\ndependencies = [\n \"chrono\",\n]\n\n[[package]]\nname = \"conveyor-macros\"\nversion = \"0.3.1\"\n";

const CHANGELOG: &str = "# Changelog\n\n## v0.3.1 (2026-11-02)\n- Fixed the builds.\n\n## v0.3.0 (2026-11-01)\n- A first key.\n";
