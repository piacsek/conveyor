use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{committed, fragment, git, isolated, released, stderr, stdout, which};

#[test]
fn a_fix_bumps_the_patch() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.3.2\n");
}

#[test]
fn a_feat_bumps_the_minor_and_resets_the_patch() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: feat\n\n- A new key.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.4.0\n");
}

#[test]
fn the_highest_kind_among_the_pending_fragments_wins() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "a-fix", "kind: fix\n\n- Fixed the queue.\n");
    fragment(&work, "b-feat", "kind: feat\n\n- A new key.\n");
    fragment(&work, "c-internal", "kind: internal\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.4.0\n");
}

#[test]
fn nothing_pending_prints_nothing_and_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_fragment_the_last_tag_already_holds_is_not_pending() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "released", "kind: feat\n\n- A new key.\n");
    git(&work, &["tag", "v0.4.0"]);

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn only_the_fragments_added_since_the_last_tag_pick_the_bump() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "released", "kind: feat\n\n- A new key.\n");
    git(&work, &["tag", "v0.4.0"]);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.4.1\n");
}

#[test]
fn a_repository_without_a_version_tag_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    git(&work, &["tag", "-d", "v0.3.1"]);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "no vX.Y.Z tag to count from; fetch the tags (git fetch --tags)\n"
    );
}

#[test]
fn a_tag_that_is_not_a_version_is_not_the_last_release() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    git(&work, &["tag", "v9-draft"]);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.3.2\n");
}

#[test]
fn the_last_release_is_the_highest_tag_by_version_not_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.9.0");
    git(&work, &["tag", "v0.10.0"]);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.10.1\n");
}

#[test]
fn a_fragment_that_is_not_committed_is_not_pending() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fs::create_dir_all(work.join("changes")).unwrap();
    fs::write(work.join("changes/topic.md"), "kind: fix\n\n- Fixed.\n").unwrap();

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_fragment_with_an_unknown_kind_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: major\n\n- A new config format.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md must start with `kind: fix`, `kind: feat` or `kind: internal`\n"
    );
}

#[test]
fn a_carriage_return_inside_a_fragment_line_is_refused_by_name_without_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: fix\n\n- Fixed the\rqueue.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md holds a carriage return inside a line: a carriage return may only end a line, right before its newline\n"
    );
}

#[test]
fn a_fragment_without_a_kind_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "a-fix", "kind: fix\n\n- Fixed the queue.\n");
    fragment(&work, "b-bare", "- Fixed the builds.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/b-bare.md must start with `kind: fix`, `kind: feat` or `kind: internal`\n"
    );
}

#[test]
fn a_fix_without_a_bullet_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: fix\n\nFixed the queue.\n- \n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md is a `fix` without a `- ` bullet\n"
    );
}

#[test]
fn a_feat_without_a_bullet_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: feat\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md is a `feat` without a `- ` bullet\n"
    );
}

#[test]
fn a_fix_with_a_star_bullet_after_its_first_bullet_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(
        &work,
        "topic",
        "kind: fix\n\n- Fixed the queue.\n* Fixed the builds.\n",
    );

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md has a bullet, so every line after its kind must be a `- ` bullet, an indented continuation or blank: * Fixed the builds.\n"
    );
}

#[test]
fn a_fix_with_prose_before_its_first_bullet_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(
        &work,
        "topic",
        "kind: fix\nIntro prose lost\n- Fixed the queue.\n",
    );

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md has a bullet, so every line after its kind must be a `- ` bullet, an indented continuation or blank: Intro prose lost\n"
    );
}

#[test]
fn an_internal_fragment_without_a_bullet_bumps_the_patch() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: internal\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.3.2\n");
}

#[test]
fn a_bullet_of_an_older_release_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    committed(
        &work,
        "CHANGELOG.md",
        "# Changelog\n\n## v0.3.2 (2026-11-02)\n- Fixed the builds.\n",
    );
    git(&work, &["tag", "v0.3.2"]);
    fragment(&work, "topic", "kind: fix\n\n- Fixed the builds.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md repeats a bullet of v0.3.2: Fixed the builds.\n"
    );
}

#[test]
fn an_internal_fragment_of_prose_only_bumps_the_patch() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(
        &work,
        "topic",
        "kind: internal\n\nThe stages load in parallel.\n* No user sees it.\n",
    );

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.3.2\n");
}

#[test]
fn a_file_in_a_folder_under_changes_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    committed(&work, "changes/fix/sub.md", "kind: feat\n\n- A new key.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/fix/sub.md is not a fragment: a fragment is a file changes/<name>.md\n"
    );
}

#[test]
fn a_file_under_changes_without_the_md_extension_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    committed(&work, "changes/UPPER.MD", "kind: feat\n\n- A new key.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/UPPER.MD is not a fragment: a fragment is a file changes/<name>.md\n"
    );
}

#[test]
fn a_fragment_name_git_quotes_is_refused_as_git_prints_it() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    committed(&work, "changes/a\"b.md", "kind: feat\n\n- A new key.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "\"changes/a\\\"b.md\" is not a fragment: a fragment is a file changes/<name>.md\n"
    );
}

#[test]
fn a_released_fragment_edited_since_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");
    git(&work, &["tag", "v0.3.2"]);
    fragment(&work, "topic", "kind: feat\n\n- A new key.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "changes/topic.md was released in v0.3.2 and has changed since; a released fragment never changes, so put the change in a new file\n"
    );
}

#[test]
fn a_released_fragment_deleted_since_is_fine() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "released", "kind: fix\n\n- Fixed the queue.\n");
    git(&work, &["tag", "v0.3.2"]);
    git(&work, &["rm", "-q", "changes/released.md"]);
    git(&work, &["commit", "-qm", "drop released fragments"]);
    fragment(&work, "topic", "kind: feat\n\n- A new key.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.4.0\n");
}

#[test]
fn a_last_tag_whose_tree_cannot_be_read_stops_the_script() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    committed(&work, "unique.txt", "only in v0.4.0\n");
    git(&work, &["tag", "v0.4.0"]);
    git(&work, &["reset", "-q", "--hard", "HEAD~1"]);
    lose_the_tree_of(&work, "v0.4.0");
    fragment(&work, "topic", "kind: fix\n\n- Fixed the queue.\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_feat_with_crlf_line_endings_bumps_the_minor() {
    let dir = tempfile::tempdir().unwrap();
    let work = released(dir.path(), "v0.3.1");
    fragment(&work, "topic", "kind: feat\r\n\r\n- A new key.\r\n");

    let output = next_version(&work);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.4.0\n");
}

#[test]
fn the_fragments_committed_in_this_repository_are_well_formed() {
    let output = next_version(Path::new(env!("CARGO_MANIFEST_DIR")));

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

fn next_version(work: &Path) -> Output {
    isolated(Command::new(which("bash")).arg(work.join("scripts/next-version.sh")))
        .output()
        .unwrap()
}

fn lose_the_tree_of(work: &Path, tag: &str) {
    let output = isolated(
        Command::new("git")
            .current_dir(work)
            .args(["rev-parse", &format!("{tag}^{{tree}}")]),
    )
    .output()
    .unwrap();
    let tree = stdout(&output);
    let (folder, file) = tree.trim().split_at(2);
    fs::remove_file(work.join(".git/objects").join(folder).join(file)).unwrap();
}
