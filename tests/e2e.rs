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
        Self::sized(name, "12")
    }

    fn sized(name: &str, height: &str) -> Self {
        let server = Self {
            socket: format!("conveyor-e2e-{name}-{}", std::process::id()),
        };
        let status = server
            .tmux(&["new-session", "-d", "-s", "live", "-x", "160", "-y", height])
            .status()
            .unwrap_or_else(|err| {
                panic!("the e2e tests drive a scratch tmux server; install tmux (brew install tmux) and rerun `cargo test -- --ignored`: {err}")
            });
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

    fn send_keys(&self, keys: &[&str]) {
        for key in keys {
            let status = self.tmux(&["send-keys", key]).status().unwrap();
            assert!(status.success());
            sleep(Duration::from_millis(120));
        }
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

fn deploy_config(home: &Path) -> String {
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
"#,
    )
    .unwrap();
    path.display().to_string()
}

fn kubectl_shim(home: &Path) {
    let kubectl = home.join("bin").join("kubectl");
    fs::write(
        &kubectl,
        "#!/bin/sh\ncase \"$*\" in *ctx-prod*) echo 'ERROR: Active profile expired.' >&2; exit 1;; *) printf 'ghcr.io/acme/api:b0b5136548f2d7c37a7c09c03e39e34aadad16bd';; esac\n",
    )
    .unwrap();
    fs::set_permissions(&kubectl, fs::Permissions::from_mode(0o755)).unwrap();
}

fn dispatching_shim(home: &Path) -> String {
    fs::write(home.join("prs.json"), include_str!("fixtures/prs.json")).unwrap();
    fs::write(home.join("queue.json"), include_str!("fixtures/queue.json")).unwrap();
    fs::write(home.join("runs.json"), include_str!("fixtures/runs.json")).unwrap();
    fs::write(
        home.join("pulls.json"),
        include_str!("fixtures/commit-pulls.json"),
    )
    .unwrap();
    fs::write(home.join("jobs.json"), include_str!("fixtures/jobs.json")).unwrap();
    fs::write(
        home.join("workflows.json"),
        r#"{"workflows":[{"id":22,"name":"CI/CD","path":".github/workflows/cicd.yml"}]}"#,
    )
    .unwrap();
    let dir = home.display();
    shim(
        home,
        &format!(
            "case \"$*\" in \
             *'repository(owner'*) cat '{dir}/queue.json';; \
             *'repo view'*) echo acme/webapp;; \
             *'actions/runs/'*'/jobs'*) cat '{dir}/jobs.json';; \
             *'actions/workflows?'*) cat '{dir}/workflows.json';; \
             *'actions/workflows/'*) cat '{dir}/runs.json';; \
             *'/commits/'*) cat '{dir}/pulls.json';; \
             *'actions/runs?'*) echo '{{\"workflow_runs\":[]}}';; \
             *) cat '{dir}/prs.json';; esac"
        ),
    )
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

#[test]
#[ignore]
fn the_queue_column_fills_from_the_checked_out_repository() {
    let home = tempfile::tempdir().unwrap();
    let path = dispatching_shim(home.path());
    let server = Server::start("queue");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    let screen = server.wait_for_screen("Queue webapp (2)");
    assert!(screen.contains("#4821 alice  Retry webhook"), "{screen}");
    assert!(screen.contains("#1 conveyor  Phase 0: setup"), "{screen}");
}

#[test]
#[ignore]
fn the_main_builds_column_fills_from_the_workflow_runs() {
    let home = tempfile::tempdir().unwrap();
    let path = dispatching_shim(home.path());
    let server = Server::start("builds");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    let screen = server.wait_for_screen("Main webapp (4)");
    assert!(screen.contains("✗ #3 piacsek  Phase 2"), "{screen}");
    assert!(screen.contains("Queue webapp (2)"), "{screen}");
}

#[test]
#[ignore]
fn the_deployed_column_fills_from_kubectl_per_environment() {
    let home = tempfile::tempdir().unwrap();
    let path = dispatching_shim(home.path());
    kubectl_shim(home.path());
    let config = deploy_config(home.path());
    let server = Server::start("deployed");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
            ("CONVEYOR_CONFIG", &config),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    let screen = server.wait_for_screen("Deployed api (2)");
    assert!(screen.contains("staging  #3 piacsek"), "{screen}");
    assert!(screen.contains("prod  ERROR: Active profile"), "{screen}");
}

#[test]
#[ignore]
fn p_on_a_main_build_lists_the_runs_jobs() {
    let home = tempfile::tempdir().unwrap();
    let path = dispatching_shim(home.path());
    let server = Server::sized("jobs", "30");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("Main webapp (4)");
    server.send_keys(&["l", "l", "p"]);

    let screen = server.wait_for_screen("failed at: Run cargo test");
    assert!(screen.contains("✗ check"), "{screen}");
    assert!(screen.contains("✓ audit"), "{screen}");
}
