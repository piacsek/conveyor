use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use super::{Server, fixture_path, shim, wait_for_file_line};

#[test]
#[ignore]
fn every_my_prs_tab_is_fetched_before_any_key() {
    let home = tempfile::tempdir().unwrap();
    let (path, argv) = logging_shim(home.path());
    let config = two_tabs_config(home.path(), "");
    let server = Server::start("prs-background-tabs");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
            ("CONVEYOR_CONFIG", &config),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    let log = wait_for_file_line(&argv, "q=is:pr review-requested:@me");
    assert!(log.contains("q=is:pr author:@me"), "{log}");
}

#[test]
#[ignore]
fn with_background_tabs_off_only_the_active_tab_is_fetched() {
    let home = tempfile::tempdir().unwrap();
    let (path, argv) = logging_shim(home.path());
    let config = two_tabs_config(home.path(), "background_tabs = false\n");
    let server = Server::start("prs-background-tabs-off");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
            ("CONVEYOR_CONFIG", &config),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");
    sleep(Duration::from_secs(1));

    let log = fs::read_to_string(&argv).unwrap();
    assert!(log.contains("q=is:pr author:@me"), "{log}");
    assert!(!log.contains("q=is:pr review-requested:@me"), "{log}");
}

#[test]
#[ignore]
fn main_builds_looks_up_the_workflows_of_every_repository_before_any_key() {
    let home = tempfile::tempdir().unwrap();
    let (path, argv) = logging_shim(home.path());
    let config = home.path().join("conveyor.toml");
    fs::write(
        &config,
        "[[repo]]\nname = \"acme/webapp\"\n\n[[repo]]\nname = \"acme/infra\"\n",
    )
    .unwrap();
    let server = Server::start("builds-background-repos");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
            ("CONVEYOR_CONFIG", &config.display().to_string()),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    wait_for_file_line(&argv, "repos/acme/webapp/actions/workflows?");
    wait_for_file_line(&argv, "repos/acme/infra/actions/workflows?");
}

fn logging_shim(home: &Path) -> (String, std::path::PathBuf) {
    let fixture = fixture_path(home);
    let argv = home.join("argv.log");
    let path = shim(
        home,
        &format!("echo \"$*\" >> '{}'; cat '{fixture}'", argv.display()),
    );
    (path, argv)
}

fn two_tabs_config(home: &Path, prs: &str) -> String {
    let path = home.join("conveyor.toml");
    fs::write(
        &path,
        format!(
            "[prs]\n{prs}\n[[prs.tab]]\nname = \"mine\"\nquery = \"is:pr author:@me\"\n\n[[prs.tab]]\nname = \"review\"\nquery = \"is:pr review-requested:@me\"\n"
        ),
    )
    .unwrap();
    path.display().to_string()
}
