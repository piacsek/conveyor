use std::path::{Path, PathBuf};

use conveyor::log::{Log, path};

#[test]
fn the_log_path_prefers_the_override_then_xdg_state_then_home() {
    let home = Path::new("/home/me");

    assert_eq!(
        path(
            Some(PathBuf::from("/x/c.log")),
            Some(PathBuf::from("/state")),
            home
        ),
        PathBuf::from("/x/c.log")
    );
    assert_eq!(
        path(None, Some(PathBuf::from("/state")), home),
        PathBuf::from("/state/conveyor/conveyor.log")
    );
    assert_eq!(
        path(None, None, home),
        PathBuf::from("/home/me/.local/state/conveyor/conveyor.log")
    );
}

#[test]
fn a_line_lands_in_the_file_under_a_directory_created_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("state").join("conveyor.log");

    let log = Log::open(&file);
    log.line("gh api user 12ms ok");

    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.ends_with(" gh api user 12ms ok\n"), "{text}");
}

#[test]
fn opening_keeps_the_previous_session_as_dot_one_and_starts_empty() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let previous = dir.path().join("conveyor.log.1");
    std::fs::write(&previous, "older session\n").unwrap();
    std::fs::write(&file, "last session\n").unwrap();

    Log::open(&file).line("new session");

    let rotated = std::fs::read_to_string(&previous).unwrap();
    let current = std::fs::read_to_string(&file).unwrap();
    assert_eq!(rotated, "last session\n");
    assert!(current.ends_with(" new session\n"), "{current}");
    assert!(!current.contains("last session"), "{current}");
}

#[test]
fn a_log_that_cannot_be_opened_drops_lines_instead_of_failing() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("not-a-dir");
    std::fs::write(&blocker, "").unwrap();

    Log::open(&blocker.join("conveyor.log")).line("dropped");

    let blocker_text = std::fs::read_to_string(&blocker).unwrap();
    assert_eq!(blocker_text, "");
}
