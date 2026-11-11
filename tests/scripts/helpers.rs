use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn calls(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

pub fn link_script(root: &Path, name: &str) {
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name);
    symlink(source, scripts.join(name)).unwrap();
}

pub fn recorder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/record-call.sh")
}

pub fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} is not on PATH"))
}

pub fn repository(root: &Path) -> PathBuf {
    let work = root.join("work");
    fs::create_dir_all(root.join("home")).unwrap();
    fs::create_dir_all(work.join("src")).unwrap();
    fs::create_dir_all(work.join("tests")).unwrap();
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["config", "user.name", "Test"]);
    git(&work, &["config", "user.email", "test@example.com"]);
    fs::write(work.join(".gitignore"), "/target\n").unwrap();
    fs::write(work.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(work.join("CHANGELOG.md"), "## v0.1.0\n").unwrap();
    fs::write(work.join("README.md"), "readme\n").unwrap();
    fs::write(work.join("AGENTS.md"), "agents\n").unwrap();
    link_script(&work, "dev-install-skippable.sh");
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "initial"]);
    git(&work, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&work, &["checkout", "-qb", "topic"]);
    work
}

pub fn git(dir: &Path, args: &[&str]) {
    let status = isolated(Command::new("git").current_dir(dir).args(args))
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

pub fn isolated(command: &mut Command) -> &mut Command {
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
}

pub fn another_build(root: &Path) -> PathBuf {
    build_in(&root.join("elsewhere"))
}

pub fn build_in(worktree: &Path) -> PathBuf {
    let binary = worktree.join("target/release/conveyor");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    symlink(which("true"), &binary).unwrap();
    binary
}

pub fn link_conveyor_dev(root: &Path, target: &Path) {
    let bin = root.join("home/.local/bin");
    fs::create_dir_all(&bin).unwrap();
    symlink(target, bin.join("conveyor-dev")).unwrap();
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
