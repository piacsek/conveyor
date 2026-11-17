use std::fs;

use super::Server;

#[test]
#[ignore]
fn the_demo_draws_my_prs_with_no_gh_and_no_config() {
    let home = tempfile::tempdir().unwrap();
    let server = Server::sized("demo-prs", "30");

    demo(&server, home.path());

    let screen = server.wait_for_screen("🔁 Retry webhook");
    assert!(screen.contains("#4830 Add rate limit"), "{screen}");
}

#[test]
#[ignore]
fn the_demo_fills_all_four_columns_with_no_gh_no_kubectl_and_no_config() {
    let home = tempfile::tempdir().unwrap();
    let server = Server::sized("demo-columns", "40");

    demo(&server, home.path());

    let screen = server.wait_for_screen("CrashLoopBackOff");
    assert!(screen.contains("🔁 Retry webhook"), "{screen}");
    assert!(screen.contains("Speed up CI with a warm cache"), "{screen}");
    assert!(screen.contains("carol"), "{screen}");
    assert!(screen.contains("Active profile expired"), "{screen}");
}

#[test]
#[ignore]
fn the_demo_says_so_in_the_footer() {
    let home = tempfile::tempdir().unwrap();
    let server = Server::sized("demo-marker", "30");

    demo(&server, home.path());

    let screen = server.wait_for_screen("🔁 Retry webhook");
    assert!(screen.contains("demo \u{25aa}"), "{screen}");
}

#[test]
#[ignore]
fn the_demo_says_so_under_the_help_pane() {
    let home = tempfile::tempdir().unwrap();
    let server = Server::sized("demo-help", "30");
    demo(&server, home.path());
    server.wait_for_screen("🔁 Retry webhook");

    server.send_keys(&["?"]);

    let screen = server.wait_for_screen("any key returns");
    assert!(screen.contains("demo"), "{screen}");
}

#[test]
#[ignore]
fn rerunning_the_failed_checks_of_a_demo_pull_request_changes_nothing() {
    let home = tempfile::tempdir().unwrap();
    let server = Server::sized("demo-rerun", "30");
    demo(&server, home.path());
    server.wait_for_screen("🔁 Retry webhook");
    server.send_keys(&["j"]);

    server.send_keys(&["S"]);
    server.wait_for_screen("Manage webapp#4821");
    server.send_keys(&["f"]);

    server.wait_for_screen("demo: nothing was changed");
}

#[test]
#[ignore]
fn opening_a_pull_request_in_the_demo_opens_nothing() {
    let home = tempfile::tempdir().unwrap();
    let server = Server::sized("demo-open", "30");
    demo(&server, home.path());
    server.wait_for_screen("🔁 Retry webhook");

    server.send_keys(&["p"]);

    server.wait_for_screen("demo: acme/* is made up, so nothing was opened");
}

fn demo(server: &Server, home: &std::path::Path) {
    let empty = home.join("empty-bin");
    fs::create_dir_all(&empty).unwrap();
    server.respawn_with_args(
        &[
            ("HOME", &home.display().to_string()),
            ("PATH", &empty.display().to_string()),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
        &["--demo"],
    );
}
