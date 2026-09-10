use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

struct Server {
    socket: String,
}

impl Server {
    fn start(name: &str) -> Self {
        let server = Self {
            socket: format!("conveyor-e2e-{name}-{}", std::process::id()),
        };
        let status = server
            .tmux(&["new-session", "-d", "-s", "live", "-x", "160", "-y", "12"])
            .status()
            .expect("tmux binary on PATH");
        assert!(status.success(), "could not start tmux e2e server");
        server
    }

    fn tmux(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("tmux");
        cmd.arg("-L")
            .arg(&self.socket)
            .arg("-f")
            .arg("/dev/null")
            .args(args);
        cmd
    }

    fn respawn(&self, env: &[(&str, &str)], command: &str) {
        let pairs: Vec<String> = env.iter().map(|(k, v)| format!("{k}='{v}'")).collect();
        let command = format!("env {} '{command}'", pairs.join(" "));
        let status = self
            .tmux(&["respawn-pane", "-k", &command])
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn wait_for_screen(&self, needle: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut screen = String::new();
        while Instant::now() < deadline {
            let out = self.tmux(&["capture-pane", "-p"]).output().unwrap();
            screen = String::from_utf8_lossy(&out.stdout).to_string();
            if screen.contains(needle) {
                return screen;
            }
            sleep(Duration::from_millis(100));
        }
        panic!("never saw {needle:?} on screen:\n{screen}");
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.tmux(&["kill-server"]).status();
    }
}

fn shim(home: &Path, script: &str) -> String {
    let bin = home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(&gh, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

fn fixture_path(home: &Path) -> String {
    let path = home.join("prs.json");
    fs::write(&path, include_str!("fixtures/prs.json")).unwrap();
    path.display().to_string()
}

#[test]
#[ignore]
fn the_binary_draws_my_prs_from_gh_before_any_key() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let path = shim(home.path(), &format!("cat '{fixture}'"));
    let server = Server::start("prs");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    let screen = server.wait_for_screen("#1 conveyor  Phase 0: setup");
    assert!(screen.contains("My PRs (1)"), "{screen}");
    assert!(screen.contains("Merge queue"), "{screen}");
}

#[test]
#[ignore]
fn a_failing_gh_shows_in_the_column_and_the_binary_keeps_running() {
    let home = tempfile::tempdir().unwrap();
    let path = shim(
        home.path(),
        "echo 'gh: HTTP 401: Bad credentials' >&2; exit 1",
    );
    let server = Server::start("fail");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    let screen = server.wait_for_screen("gh: HTTP 401: Bad credentials");
    assert!(
        screen.contains("My PRs ⚠") || screen.contains("My PRs (0) ⚠"),
        "{screen}"
    );
    sleep(Duration::from_millis(500));
    let again = server.wait_for_screen("Bad credentials");
    assert!(again.contains("Deployed"), "still running: {again}");
}
