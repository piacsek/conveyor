use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

static SHIMS: Mutex<()> = Mutex::new(());

#[test]
fn gates_run_every_test_in_one_pass_when_tmux_is_installed() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let bin = gates_sandbox(dir.path());
    symlink(which("true"), bin.join("tmux")).unwrap();

    let output = gates(dir.path());

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        cargo_calls(dir.path()),
        [
            "fmt --check",
            "clippy --all-targets -- -D warnings",
            "test -- --include-ignored",
        ]
    );
}

#[test]
fn gates_without_tmux_run_the_plain_tests_and_say_what_they_skipped() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    gates_sandbox(dir.path());

    let output = gates(dir.path());

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        cargo_calls(dir.path()),
        ["fmt --check", "clippy --all-targets -- -D warnings", "test"]
    );
    assert!(
        stderr(&output).contains("skipping the ignored e2e tests: tmux is not installed"),
        "{}",
        stderr(&output)
    );
}

fn serialized() -> MutexGuard<'static, ()> {
    SHIMS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn gates_sandbox(root: &Path) -> PathBuf {
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    symlink(which("bash"), bin.join("bash")).unwrap();
    symlink(which("dirname"), bin.join("dirname")).unwrap();
    executable(
        &bin.join("cargo"),
        &format!("echo \"$*\" >> '{}'", root.join("cargo.log").display()),
    );
    copy_script(root, "gates.sh");
    executable(&root.join("scripts/scrub-check.sh"), "exit 0");
    bin
}

fn gates(root: &Path) -> Output {
    Command::new(root.join("scripts/gates.sh"))
        .env_clear()
        .env("PATH", root.join("bin"))
        .output()
        .unwrap()
}

fn cargo_calls(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join("cargo.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn copy_script(root: &Path, name: &str) {
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name);
    fs::copy(source, scripts.join(name)).unwrap();
}

fn executable(path: &Path, script: &str) {
    fs::write(path, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} is not on PATH"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
