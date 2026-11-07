use std::path::{Path, PathBuf};

use conveyor::log::{Log, path, program_name};

#[test]
fn the_log_path_prefers_the_override_then_xdg_state_then_home() {
    let home = Path::new("/home/me");

    assert_eq!(
        path(
            Some(PathBuf::from("/x/c.log")),
            Some(PathBuf::from("/state")),
            home,
            "conveyor"
        ),
        Some(PathBuf::from("/x/c.log"))
    );
    assert_eq!(
        path(None, Some(PathBuf::from("/state")), home, "conveyor"),
        Some(PathBuf::from("/state/conveyor/conveyor.log"))
    );
    assert_eq!(
        path(None, None, home, "conveyor"),
        Some(PathBuf::from("/home/me/.local/state/conveyor/conveyor.log"))
    );
}

#[test]
fn a_relative_or_empty_base_never_puts_the_log_under_the_working_directory() {
    let home = Path::new("/home/me");

    assert_eq!(
        path(Some(PathBuf::from("")), None, home, "conveyor"),
        Some(PathBuf::from("/home/me/.local/state/conveyor/conveyor.log"))
    );
    assert_eq!(
        path(None, Some(PathBuf::from("rel")), home, "conveyor"),
        Some(PathBuf::from("/home/me/.local/state/conveyor/conveyor.log"))
    );
    assert_eq!(path(None, None, Path::new(""), "conveyor"), None);
    assert_eq!(path(None, None, Path::new("rel"), "conveyor"), None);
}

#[test]
fn each_program_writes_its_own_log() {
    let home = Path::new("/home/me");

    assert_eq!(
        path(None, None, home, "conveyor-dev"),
        Some(PathBuf::from(
            "/home/me/.local/state/conveyor/conveyor-dev.log"
        ))
    );
}

#[test]
fn the_program_name_is_the_file_name_it_was_started_as() {
    assert_eq!(
        program_name(Some("/home/me/.local/bin/conveyor-dev".into())),
        "conveyor-dev"
    );
    assert_eq!(program_name(Some("conveyor".into())), "conveyor");
    assert_eq!(program_name(None), "conveyor");
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

#[test]
fn the_log_is_readable_by_its_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");

    Log::open(&file).line("private");

    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}
