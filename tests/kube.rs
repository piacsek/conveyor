use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use conveyor::config::DeployEnv;
use conveyor::log::Log;
use conveyor::model::deployed::sha_from_image;
use conveyor::sources::kube::{CliKubectl, Kube};

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
    let kubectl = shim(&dir, "sleep 5")
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
fn a_helper_that_outlives_kubectl_and_holds_its_output_is_a_timeout_too() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let kubectl = shim(&dir, "sleep 5 &\necho '{}'").with_deadline(Duration::from_secs(1));
    let started = Instant::now();

    let err = kubectl.deployment(&env()).unwrap_err();

    assert!(started.elapsed() < Duration::from_secs(3), "{err}");
    assert_eq!(err.to_string(), "kubectl did not answer within 1s");
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
