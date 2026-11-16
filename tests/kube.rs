use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::{Arc, Barrier, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use conveyor::app::{App, Rows, Stage};
use conveyor::config::{Config, Deploy, DeployEnv};
use conveyor::log::Log;
use conveyor::model::deployed::sha_from_image;
use conveyor::sources::failure::FailureKind;
use conveyor::sources::fetch::DeploySource;
use conveyor::sources::github::CliGh;
use conveyor::sources::kube::{CliKubectl, Kube, MISSING};

static SHIMS: Mutex<()> = Mutex::new(());

fn serialized() -> MutexGuard<'static, ()> {
    SHIMS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn shim(dir: &tempfile::TempDir, script: &str) -> CliKubectl {
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let kubectl = bin.join("kubectl");
    fs::write(&kubectl, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&kubectl, fs::Permissions::from_mode(0o755)).unwrap();
    CliKubectl::with_program(kubectl)
}

fn env() -> DeployEnv {
    DeployEnv {
        name: "prod".to_string(),
        context: "teleport-prod".to_string(),
        namespace: "api".to_string(),
        deployment: "api".to_string(),
        ..DeployEnv::default()
    }
}

#[test]
fn a_failing_kubectl_call_is_logged_with_its_argv_and_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let kubectl =
        shim(&dir, "echo 'error: You must be logged in' >&2; exit 1").with_log(Log::open(&file));

    let _ = kubectl.deployment(&env());

    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains(" kubectl --context teleport-prod -n api get deploy api "),
        "{text}"
    );
    assert!(
        text.ends_with("ms exit 1: error: You must be logged in\n"),
        "{text}"
    );
}

#[test]
fn a_kubectl_that_hangs_is_killed_at_the_deadline() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let kubectl = shim(&dir, "exec sleep 5")
        .with_log(Log::open(&file))
        .with_deadline(Duration::from_secs(1));
    let started = Instant::now();

    let err = kubectl.deployment(&env()).unwrap_err();

    assert!(started.elapsed() < Duration::from_secs(3), "{err}");
    assert_eq!(err.to_string(), "kubectl did not answer within 1s");
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.ends_with("ms timed out\n"), "{text}");
}

#[test]
fn a_helper_that_outlives_kubectl_does_not_cost_its_answer() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(&dir, "sleep 30 &\necho '{}'").with_deadline(Duration::from_secs(10));
    let started = Instant::now();

    let answer = kubectl.deployment(&env()).unwrap();

    assert!(started.elapsed() < Duration::from_secs(3), "{answer}");
    assert_eq!(answer, "{}");
}

#[test]
fn a_failure_shows_what_kubectl_said_without_its_klog_lines() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(
        &dir,
        r#"echo 'E1109 20:36:29.585690   14272 memcache.go:265] "Unhandled Error" err="could not get current server API group list"' >&2
echo 'Unable to connect to the server: context deadline exceeded' >&2
exit 1"#,
    );

    let err = kubectl.deployment(&env()).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Unable to connect to the server: context deadline exceeded"
    );
}

#[test]
fn a_failure_that_is_only_klog_lines_still_shows_them() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(
        &dir,
        r#"echo 'E1109 20:36:29.585690   14272 memcache.go:265] "Unhandled Error" err="no answer"' >&2
exit 1"#,
    );

    let err = kubectl.deployment(&env()).unwrap_err();

    assert_eq!(
        err.to_string(),
        r#"E1109 20:36:29.585690   14272 memcache.go:265] "Unhandled Error" err="no answer""#
    );
}

#[test]
fn a_helper_kubectl_started_is_killed_with_it_at_the_deadline() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(
        &dir,
        "sleep 30 &\necho $! > \"$0.pid\"\n: > \"$0.ready\"\nwait",
    );

    time_out_once_ready(&dir, kubectl);

    assert!(!alive_after(&dir, Duration::from_secs(2)));
}

#[test]
fn a_helper_that_outlives_kubectl_is_killed_once_its_output_is_given_up_on() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(&dir, "sleep 30 &\necho $! > \"$0.pid\"\necho '{}'")
        .with_deadline(Duration::from_secs(10));

    let _ = kubectl.deployment(&env());

    assert!(!alive_after(&dir, Duration::from_secs(2)));
}

#[test]
fn a_timed_out_call_is_logged_with_what_kubectl_had_said() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let kubectl = shim(
        &dir,
        "echo 'dial tcp: i/o timeout' >&2\n: > \"$0.ready\"\nexec sleep 5",
    )
    .with_log(Log::open(&file));

    time_out_once_ready(&dir, kubectl);

    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.ends_with("ms timed out: dial tcp: i/o timeout\n"),
        "{text}"
    );
}

#[test]
fn a_helper_that_outlives_a_failed_kubectl_does_not_hide_its_reason() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(
        &dir,
        "echo 'error: You must be logged in' >&2\nsleep 30 &\nexit 1",
    )
    .with_deadline(Duration::from_secs(10));
    let started = Instant::now();

    let err = kubectl.deployment(&env()).unwrap_err();

    assert!(started.elapsed() < Duration::from_secs(3), "{err}");
    assert_eq!(err.to_string(), "error: You must be logged in");
}

#[test]
fn a_call_made_while_another_hangs_still_gets_its_answer() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = Arc::new(
        shim(
            &dir,
            r#"case "$*" in *hung*) sleep 2;; *) echo '{}';; esac"#,
        )
        .with_deadline(Duration::from_secs(3)),
    );
    let mut lost = 0;

    for _ in 0..150 {
        let start = Arc::new(Barrier::new(4));
        std::thread::spawn({
            let (kubectl, start) = (kubectl.clone(), start.clone());
            move || {
                start.wait();
                let _ = kubectl.deployment(&DeployEnv {
                    context: "hung".to_string(),
                    ..env()
                });
            }
        });
        let healthy: Vec<_> = (0..3)
            .map(|_| {
                let (kubectl, start) = (kubectl.clone(), start.clone());
                std::thread::spawn(move || {
                    start.wait();
                    kubectl.deployment(&env()).is_err()
                })
            })
            .collect();
        lost += healthy
            .into_iter()
            .map(|call| call.join().unwrap())
            .filter(|lost| *lost)
            .count();
    }

    assert_eq!(lost, 0);
}

#[test]
fn deployment_and_pods_argv_ask_for_json_with_a_timeout() {
    assert_eq!(
        CliKubectl::deployment_args(&env()),
        vec![
            "--context",
            "teleport-prod",
            "-n",
            "api",
            "get",
            "deploy",
            "api",
            "-o",
            "json",
            "--request-timeout=10s",
        ]
    );
    assert_eq!(
        CliKubectl::pods_args(&env(), "app=api,tier=web"),
        vec![
            "--context",
            "teleport-prod",
            "-n",
            "api",
            "get",
            "pods",
            "-l",
            "app=api,tier=web",
            "-o",
            "json",
            "--request-timeout=10s",
        ]
    );
}

#[test]
fn deployment_and_pods_return_stdout_and_map_failures() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let ok = shim(
        &dir,
        r#"case "$*" in *"get pods -l app=api"*) printf '{"items":[]}';; *) printf '{"kind":"Deployment"}';; esac"#,
    );
    assert_eq!(ok.deployment(&env()).unwrap(), r#"{"kind":"Deployment"}"#);
    assert_eq!(ok.pods(&env(), "app=api").unwrap(), r#"{"items":[]}"#);

    let dir = tempfile::tempdir().unwrap();
    let failing = shim(&dir, "echo 'ERROR: Active profile expired.' >&2; exit 1");
    assert_eq!(
        failing.deployment(&env()).unwrap_err().to_string(),
        "ERROR: Active profile expired."
    );
    assert_eq!(
        failing.pods(&env(), "app=api").unwrap_err().to_string(),
        "ERROR: Active profile expired."
    );

    let missing = CliKubectl::with_program("/nonexistent/kubectl".into());
    assert_eq!(
        missing.pods(&env(), "app=api").unwrap_err().to_string(),
        "kubectl not found on PATH; install kubectl and log in to the cluster"
    );
}

#[test]
fn a_failing_pods_call_is_logged_with_its_argv_and_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let kubectl =
        shim(&dir, "echo 'Error from server (Forbidden)' >&2; exit 1").with_log(Log::open(&file));

    let _ = kubectl.pods(&env(), "app=api");

    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains(" kubectl --context teleport-prod -n api get pods -l app=api "),
        "{text}"
    );
    assert!(
        text.ends_with("ms exit 1: Error from server (Forbidden)\n"),
        "{text}"
    );
}

#[test]
fn the_sha_is_the_40_hex_tag_or_a_40_hex_suffix() {
    assert_eq!(
        sha_from_image("ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567").unwrap(),
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(
        sha_from_image("ghcr.io/acme/api:preview-0123456789abcdef0123456789abcdef01234567")
            .unwrap(),
        "0123456789abcdef0123456789abcdef01234567"
    );
    let err = sha_from_image("ghcr.io/acme/api:latest").unwrap_err();
    assert!(
        err.contains("unexpected image tag") && err.contains("latest"),
        "{err}"
    );
    assert!(sha_from_image("ghcr.io/acme/api").is_err());
    assert!(sha_from_image("registry:5000/acme/api@sha256:abc").is_err());
}

#[test]
fn a_missing_kubectl_fails_the_deployed_row_as_program_missing_with_its_own_words() {
    let app = deployed_by(CliKubectl::with_program("/nonexistent/kubectl".into()));

    let failure = app
        .deployed
        .column
        .selected()
        .unwrap()
        .error
        .clone()
        .unwrap();
    assert_eq!(failure.kind(), FailureKind::ProgramMissing);
    assert_eq!(app.focused_error(), Some(MISSING));
}

#[test]
fn a_kubectl_past_its_deadline_fails_the_deployed_row_as_timed_out_with_its_own_words() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(&dir, "exec sleep 5").with_deadline(Duration::from_secs(1));

    let app = deployed_by(kubectl);

    let failure = app
        .deployed
        .column
        .selected()
        .unwrap()
        .error
        .clone()
        .unwrap();
    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(
        app.focused_error(),
        Some("kubectl did not answer within 1s")
    );
}

#[test]
fn any_other_kubectl_failure_fails_the_deployed_row_as_other_with_its_own_words() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(
        &dir,
        "echo 'error: You must be logged in to the server' >&2; exit 1",
    );

    let app = deployed_by(kubectl);

    let failure = app
        .deployed
        .column
        .selected()
        .unwrap()
        .error
        .clone()
        .unwrap();
    assert_eq!(failure.kind(), FailureKind::Other);
    assert_eq!(
        app.focused_error(),
        Some("error: You must be logged in to the server")
    );
}

#[test]
fn pods_past_the_deadline_fail_the_rows_pods_as_timed_out_with_their_own_words() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(
        &dir,
        &format!(
            r#"case "$*" in *"get pods"*) exec sleep 5;; *) cat '{}/tests/fixtures/deployment.json';; esac"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    );
    kubectl.deployment(&env()).unwrap();

    let app = deployed_by(kubectl.with_deadline(Duration::from_secs(1)));

    let failure = app
        .deployed
        .column
        .selected()
        .unwrap()
        .pods_error
        .clone()
        .unwrap();
    assert_eq!(failure.kind(), FailureKind::TimedOut);
    assert_eq!(failure.to_string(), "kubectl did not answer within 1s");
}

fn deployed_by(kubectl: CliKubectl) -> App {
    let deploy = Deploy {
        env: vec![env()],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/api".to_string(), deploy);
    let gh = CliGh::with_program("/nonexistent/gh".into());
    let mut app = App::new(Config::default());
    app.focus = Stage::Deployed;
    app.receive(
        Stage::Deployed,
        0,
        Ok(Rows::Deployed(source.fetch(
            &gh,
            &kubectl,
            std::time::SystemTime::UNIX_EPOCH,
        ))),
    );
    app
}

fn time_out_once_ready(dir: &tempfile::TempDir, mut kubectl: CliKubectl) {
    let ready = dir.path().join("bin/kubectl.ready");
    let mut deadline = Duration::from_millis(300);
    loop {
        kubectl = kubectl.with_deadline(deadline);
        let _ = kubectl.deployment(&env());
        if ready.exists() || deadline > Duration::from_secs(30) {
            return;
        }
        deadline *= 2;
    }
}

fn alive_after(dir: &tempfile::TempDir, patience: Duration) -> bool {
    let pid = fs::read_to_string(dir.path().join("bin/kubectl.pid")).unwrap();
    let started = Instant::now();
    while started.elapsed() < patience {
        if !alive(pid.trim()) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    alive(pid.trim())
}

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .output()
        .is_ok_and(|output| output.status.success())
}
