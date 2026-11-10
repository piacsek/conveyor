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
        Self::with_env(name, height, &[])
    }

    fn with_env(name: &str, height: &str, env: &[(&str, &str)]) -> Self {
        let server = Self {
            socket: format!("conveyor-e2e-{name}-{}", std::process::id()),
        };
        let status = server
            .tmux(&["new-session", "-d", "-s", "live", "-x", "160", "-y", height])
            .envs(env.iter().copied())
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
        let command = format!(
            "env -u XDG_CONFIG_HOME -u CONVEYOR_CONFIG -u XDG_STATE_HOME -u CONVEYOR_LOG {} '{command}'",
            pairs.join(" ")
        );
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
    let deployment = home.join("deployment.json");
    fs::write(
        &deployment,
        r#"{"spec":{"replicas":1,"selector":{"matchLabels":{"app":"api"}},"template":{"spec":{"containers":[{"image":"ghcr.io/acme/api:b0b5136548f2d7c37a7c09c03e39e34aadad16bd"}]}}},"status":{"availableReplicas":1,"conditions":[{"type":"Available","status":"True"}]}}"#,
    )
    .unwrap();
    let pods = home.join("pods.json");
    fs::write(&pods, include_str!("fixtures/pods.json")).unwrap();
    let kubectl = home.join("bin").join("kubectl");
    fs::write(
        &kubectl,
        format!(
            "#!/bin/sh\ncase \"$*\" in *ctx-prod*) echo 'ERROR: Active profile expired.' >&2; exit 1;; *'get pods -l app=api'*) cat '{}';; *) cat '{}';; esac\n",
            pods.display(),
            deployment.display()
        ),
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
        home.join("commit.json"),
        r#"{"commit":{"message":"Phase 2: merge queue column (#3)"}}"#,
    )
    .unwrap();
    fs::write(
        home.join("pull.json"),
        r#"{"number":3,"title":"Phase 2: merge queue column","user":{"login":"piacsek"},"html_url":"https://github.com/acme/webapp/pull/3","merge_commit_sha":"b0b513654654b311044cebdd3a61d6ccc436c6fa"}"#,
    )
    .unwrap();
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
             'run rerun '*) echo \"$*\" >> '{dir}/reruns.log';; \
             *'actions/runs/'*'/jobs'*) cat '{dir}/jobs.json';; \
             *'actions/workflows?'*) cat '{dir}/workflows.json';; \
             *'actions/workflows/'*) cat '{dir}/runs.json';; \
             *'/commits/'*'/pulls'*) cat '{dir}/pulls.json';; \
             *'/commits/'*) cat '{dir}/commit.json';; \
             *'/pulls/'*) cat '{dir}/pull.json';; \
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

    let screen = server.wait_for_screen("#1 Phase 0: setup");
    assert!(screen.contains("My PRs (1)"), "{screen}");
    assert!(screen.contains("conveyor · "), "the meta line: {screen}");
    assert!(screen.contains("Merge queue"), "{screen}");
}

#[test]
#[ignore]
fn a_session_logs_its_start_each_gh_call_and_its_quit() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let path = shim(home.path(), &format!("cat '{fixture}'"));
    let server = Server::start("log");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["q"]);

    let log = home.path().join(".local/state/conveyor/conveyor.log");
    let text = wait_for_file_line(&log, " conveyor quit");
    assert!(
        text.contains(&format!(" conveyor {} started", env!("CARGO_PKG_VERSION"))),
        "{text}"
    );
    assert!(text.contains(" gh api graphql -f query=<"), "{text}");
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
    assert!(screen.contains("#4821 Retry webhook"), "{screen}");
    assert!(screen.contains("alice · position 1"), "{screen}");
    assert!(screen.contains("#1 Phase 0: setup"), "{screen}");
}

#[test]
#[ignore]
fn the_binary_never_reads_a_config_from_the_callers_environment() {
    let home = tempfile::tempdir().unwrap();
    let path = dispatching_shim(home.path());
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(outside.path().join("conveyor")).unwrap();
    fs::write(
        outside.path().join("conveyor").join("config.toml"),
        "[[repo]]\nname = \"acme/leaked\"\n",
    )
    .unwrap();
    let outside_dir = outside.path().display().to_string();
    let outside_file = outside.path().join("conveyor").join("config.toml");
    let outside_file = outside_file.display().to_string();
    let server = Server::with_env(
        "isolation",
        "12",
        &[
            ("XDG_CONFIG_HOME", &outside_dir),
            ("CONVEYOR_CONFIG", &outside_file),
        ],
    );

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );

    let screen = server.wait_for_screen("Queue webapp (2)");
    assert!(!screen.contains("leaked"), "{screen}");
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
    let headline = screen
        .lines()
        .find(|line| line.contains("✗ #3"))
        .unwrap_or_else(|| panic!("{screen}"));
    assert!(
        !headline.contains("Phase 2"),
        "the first line is the number and the age: {screen}"
    );
    assert!(
        screen.lines().any(|line| line.contains("Phase 2")),
        "the title has a line of its own: {screen}"
    );
    assert!(screen.contains("piacsek · run "), "{screen}");
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
    assert!(screen.contains("#3 Phase 2"), "the title first: {screen}");
    assert!(screen.contains("piacsek"), "then the author: {screen}");
    assert!(screen.contains("ERROR: Active profile"), "{screen}");
}

#[test]
#[ignore]
fn d_on_a_main_build_lists_the_runs_jobs() {
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
    server.send_keys(&["l", "l", "d"]);

    let screen = server.wait_for_screen("failed at: Run cargo test");
    assert!(screen.contains("✗ check"), "{screen}");
    assert!(screen.contains("✓ audit"), "{screen}");
}

#[test]
#[ignore]
fn s_then_f_reruns_a_failed_main_build_and_any_other_key_runs_nothing() {
    let home = tempfile::tempdir().unwrap();
    let path = dispatching_shim(home.path());
    let server = Server::sized("rerun", "30");

    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("Main webapp (4)");
    server.send_keys(&["l", "l", "S"]);
    server.wait_for_screen("Manage run 24");
    server.send_keys(&["n"]);
    server.wait_for_screen("nothing changed");
    server.send_keys(&["S", "f"]);
    server.wait_for_screen("rerun requested: run 24");

    let log = fs::read_to_string(home.path().join("reruns.log")).unwrap();
    assert_eq!(log, "run rerun 34509123916 --failed -R acme/webapp\n");
}

#[test]
#[ignore]
fn s_m_then_y_runs_gh_pr_merge_pinned_to_the_head_commit() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let argv = home.path().join("merge-argv");
    let path = shim(
        home.path(),
        &format!(
            "case \"$*\" in 'pr merge'*) echo \"$*\" > '{}';; *) cat '{fixture}';; esac",
            argv.display()
        ),
    );
    let server = Server::start("merge");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["S", "m"]);
    server.wait_for_screen("merge #1 Phase 0: setup?");
    server.send_keys(&["y"]);
    server.wait_for_screen("merge sent for #1");

    assert_eq!(
        fs::read_to_string(&argv).unwrap(),
        "pr merge 1 -R piacsek/conveyor --squash --match-head-commit 54b33470c91a2ce5a25ddd4bfea8acf6c3b9d481\n"
    );
}

#[test]
#[ignore]
fn s_then_c_runs_gh_pr_checkout_in_the_configured_clone() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let argv = home.path().join("checkout-argv");
    let clone = home.path().join("clone");
    fs::create_dir_all(&clone).unwrap();
    let config = home.path().join(".config/conveyor");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("config.toml"),
        "[[repo]]\nname = \"piacsek/conveyor\"\npath = \"~/clone\"\n",
    )
    .unwrap();
    let path = shim(
        home.path(),
        &format!(
            "case \"$*\" in 'repo view'*) echo piacsek/conveyor;; 'pr checkout'*) echo \"$(pwd -P): $*\" >> '{}';; *) cat '{fixture}';; esac",
            argv.display()
        ),
    );
    let server = Server::start("checkout");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["S", "c"]);
    server.wait_for_screen("#1 checked out in ~/clone");

    assert_eq!(
        fs::read_to_string(&argv).unwrap(),
        format!(
            "{}: pr checkout 1 -R piacsek/conveyor\n",
            clone.canonicalize().unwrap().display()
        )
    );
}

#[test]
#[ignore]
fn s_then_u_runs_gh_pr_update_branch_and_n_runs_nothing() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let argv = home.path().join("update-argv");
    let path = shim(
        home.path(),
        &format!(
            "case \"$*\" in 'pr update-branch'*) echo \"$*\" >> '{}'; echo '✓ PR branch updated';; *) cat '{fixture}';; esac",
            argv.display()
        ),
    );
    let server = Server::start("update");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["S"]);
    server.wait_for_screen("Manage #1");
    server.send_keys(&["n"]);
    server.wait_for_screen("nothing changed");
    assert!(!argv.exists());

    server.send_keys(&["S", "u"]);
    server.wait_for_screen("branch of #1 updated");

    assert_eq!(
        fs::read_to_string(&argv).unwrap(),
        "pr update-branch 1 -R piacsek/conveyor\n"
    );
}

#[test]
#[ignore]
fn s_then_u_on_a_conflicting_branch_says_so() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let path = shim(
        home.path(),
        &format!(
            "case \"$*\" in 'pr update-branch'*) echo 'X Cannot update PR branch due to conflicts' >&2; exit 1;; *) cat '{fixture}';; esac"
        ),
    );
    let server = Server::start("conflict");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["S", "u"]);
    server.wait_for_screen("#1 has conflicts with its base; resolve them locally");
}

#[test]
#[ignore]
fn s_then_t_runs_gh_pr_ready_undo_on_an_open_pull_request() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let argv = home.path().join("status-argv");
    let path = shim(
        home.path(),
        &format!(
            "case \"$*\" in 'pr ready'*) echo \"$*\" >> '{}';; *) cat '{fixture}';; esac",
            argv.display()
        ),
    );
    let server = Server::start("status");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["S", "t"]);
    server.wait_for_screen("#1 converted to draft");

    assert_eq!(
        fs::read_to_string(&argv).unwrap(),
        "pr ready 1 -R piacsek/conveyor --undo\n"
    );
}

fn wait_for_file_line(path: &Path, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut text = String::new();
    while Instant::now() < deadline {
        text = fs::read_to_string(path).unwrap_or_default();
        if text.contains(needle) {
            return text;
        }
        sleep(Duration::from_millis(100));
    }
    panic!("never saw {needle:?} in {}:\n{text}", path.display());
}
