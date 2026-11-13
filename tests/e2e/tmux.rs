use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use crate::{Server, fixture_path, shim, wait_for_file_line};

fn under_tmux_answering(name: &str, answer: &str) -> (Server, tempfile::TempDir) {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let path = shim(home.path(), &format!("cat '{fixture}'"));
    let tmux = home.path().join("bin/tmux");
    fs::write(&tmux, format!("#!/bin/sh\nprintf '{answer}\\n'\n")).unwrap();
    fs::set_permissions(&tmux, fs::Permissions::from_mode(0o755)).unwrap();
    let server = Server::sized(name, "24");
    let home_dir = home.path().display().to_string();
    server.respawn(
        &[("HOME", &home_dir), ("PATH", &path)],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    (server, home)
}

#[test]
#[ignore]
fn inside_tmux_a_client_without_hyperlinks_is_told_how_to_turn_them_on() {
    let (server, home) =
        under_tmux_answering("nolinks", "xterm-ghostty\\t3.4\\tclipboard,focus,title");

    server.wait_for_screen("tmux drops the #N links: ? says why");
    server.send_keys(&["?"]);
    server.wait_for_screen("tmux set -as terminal-features ',xterm-ghostty:hyperlinks'");
    let log = wait_for_file_line(
        &log_of(&home),
        " tmux set -as terminal-features ',xterm-ghostty:hyperlinks'",
    );
    assert!(
        log.contains(
            "tmux display-message -p #{client_termname}\\t#{version}\\t#{client_termfeatures} "
        ),
        "{log}"
    );
}

#[test]
#[ignore]
fn inside_tmux_a_client_with_hyperlinks_hears_nothing_about_them() {
    let (server, _home) =
        under_tmux_answering("links", "xterm-ghostty\\t3.4\\tclipboard,hyperlinks,title");

    let screen = server.wait_for_screen("#1 Phase 0: setup");
    assert!(!screen.contains("drops the #N links"), "{screen}");
}

#[test]
#[ignore]
fn a_tmux_whose_answer_never_ends_holds_the_columns_back_at_most_a_moment() {
    let started = Instant::now();
    let (server, home) = under_tmux_answering(
        "hung",
        "xterm-ghostty\\t3.4\\ttitle\\n'; exec sleep 30; printf '",
    );

    let screen = server.wait_for_screen("#1 Phase 0: setup");
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    assert!(!screen.contains("drops the #N links"), "{screen}");
    let log = wait_for_file_line(&log_of(&home), "tmux display-message -p");
    assert!(log.contains("ms timed out"), "{log}");
}

#[test]
#[ignore]
fn a_tmux_that_leaves_its_output_open_holds_the_columns_back_at_most_a_moment() {
    let (server, _home) = under_tmux_answering(
        "held",
        "xterm-ghostty\\t3.4\\ttitle\\n'; sleep 30 & printf '",
    );

    let started = Instant::now();
    server.wait_for_screen("#1 Phase 0: setup");
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
}

fn log_of(home: &tempfile::TempDir) -> std::path::PathBuf {
    home.path().join(".local/state/conveyor/conveyor.log")
}
