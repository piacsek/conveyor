use std::fs;

use super::Server;

#[test]
#[ignore]
fn the_demo_draws_my_prs_with_no_gh_and_no_config() {
    let home = tempfile::tempdir().unwrap();
    let server = Server::sized("demo-prs", "30");

    demo(&server, home.path());

    let screen = server.wait_for_screen("#4821 Retry webhook");
    assert!(screen.contains("#4830 Add rate limit"), "{screen}");
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
