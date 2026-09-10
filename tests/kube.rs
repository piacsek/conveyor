use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, MutexGuard};

use conveyor::config::DeployEnv;
use conveyor::kube::{CliKubectl, Kube};
use conveyor::model::deployed::sha_from_image;

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
fn image_argv_targets_the_context_namespace_and_deployment_with_a_timeout() {
    assert_eq!(
        CliKubectl::image_args(&env()),
        vec![
            "--context",
            "teleport-prod",
            "-n",
            "api",
            "get",
            "deploy",
            "api",
            "-o",
            "jsonpath={.spec.template.spec.containers[0].image}",
            "--request-timeout=10s",
        ]
    );
}

#[test]
fn image_returns_the_trimmed_stdout_and_maps_failures() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let ok = shim(
        &dir,
        "printf 'ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567'",
    );
    assert_eq!(
        ok.image(&env()).unwrap(),
        "ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567"
    );

    let dir = tempfile::tempdir().unwrap();
    let failing = shim(&dir, "echo 'ERROR: Active profile expired.' >&2; exit 1");
    assert_eq!(
        failing.image(&env()).unwrap_err().to_string(),
        "ERROR: Active profile expired."
    );

    let missing = CliKubectl::with_program("/nonexistent/kubectl".into());
    assert_eq!(
        missing.image(&env()).unwrap_err().to_string(),
        "kubectl not found on PATH; install kubectl and log in to the cluster"
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
