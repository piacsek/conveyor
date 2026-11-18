use std::fs;
use std::path::Path;

use super::{Server, dispatching_shim, kubectl_shim, wait_for_file_line};

#[test]
#[ignore]
fn every_deployed_tab_fills_from_its_own_deploy_before_it_is_shown() {
    let home = tempfile::tempdir().unwrap();
    let path = dispatching_shim(home.path());
    kubectl_shim(home.path());
    let config = two_systems(home.path());
    let server = Server::start("deployed-tabs");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
            ("CONVEYOR_CONFIG", &config),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("Deployed api (2)");
    let log = home.path().join(".local/state/conveyor/conveyor.log");
    wait_for_file_line(&log, "ctx-canary");

    server.send_keys(&["l", "l", "l", "Tab"]);

    let screen = server.wait_for_screen("Deployed worker (1)");
    assert!(screen.contains("canary"), "{screen}");
}

fn two_systems(home: &Path) -> String {
    let path = home.join("conveyor.toml");
    fs::write(
        &path,
        r#"[[repo]]
name = "acme/webapp"

[[repo.deploy]]
system = "api"

[[repo.deploy.env]]
name = "staging"
context = "ctx-staging"
namespace = "api"
deployment = "api"

[[repo.deploy.env]]
name = "prod"
context = "ctx-prod"
namespace = "api"
deployment = "api"

[[repo.deploy]]
system = "worker"

[[repo.deploy.env]]
name = "canary"
context = "ctx-canary"
namespace = "worker"
deployment = "worker"
"#,
    )
    .unwrap();
    path.display().to_string()
}
