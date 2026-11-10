use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[test]
fn gates_run_every_test_in_one_pass_when_tmux_is_installed() {
    let dir = tempfile::tempdir().unwrap();
    let bin = gates_sandbox(dir.path());
    symlink(which("true"), bin.join("tmux")).unwrap();

    let output = gates(dir.path());

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "scrub-check.sh ",
            "cargo fmt --check",
            "cargo clippy --all-targets -- -D warnings",
            "cargo test -- --include-ignored",
        ]
    );
}

#[test]
fn gates_without_tmux_run_the_plain_tests_and_say_what_they_skipped() {
    let dir = tempfile::tempdir().unwrap();
    gates_sandbox(dir.path());

    let output = gates(dir.path());

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "scrub-check.sh ",
            "cargo fmt --check",
            "cargo clippy --all-targets -- -D warnings",
            "cargo test",
        ]
    );
    assert!(
        stderr(&output).contains("skipping the ignored e2e tests: tmux is not installed"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_branch_that_feeds_nothing_into_the_binary_can_skip_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("README.md"), "changed\n").unwrap();
    fs::write(work.join("tests/new.rs"), "").unwrap();
    git(&work, &["commit", "-qam", "docs"]);
    fs::write(work.join("AGENTS.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "nothing on this branch feeds the binary, and conveyor-dev is another worktree's build\n"
    );
}

#[test]
fn an_untracked_source_file_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("src/new.rs"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/new.rs feeds the binary\n");
}

#[test]
fn the_changelog_is_bundled_so_editing_it_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("CHANGELOG.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "CHANGELOG.md feeds the binary\n");
}

#[test]
fn a_path_nobody_listed_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("build.rs"), "").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "build.rs feeds the binary\n");
}

#[test]
fn a_source_change_committed_earlier_on_the_branch_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("src/main.rs"), "fn main() { }\n").unwrap();
    git(&work, &["commit", "-qam", "source"]);
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/main.rs feeds the binary\n");
}

#[test]
fn a_missing_conveyor_dev_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    repository(dir.path());

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "conveyor-dev is missing or dangling\n");
}

#[test]
fn a_dangling_conveyor_dev_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    repository(dir.path());
    link_conveyor_dev(dir.path(), &dir.path().join("removed/conveyor"));

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "conveyor-dev is missing or dangling\n");
}

#[test]
fn a_conveyor_dev_built_here_always_goes_through_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &build_in(&work));

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "conveyor-dev is this worktree's build\n");
}

#[test]
fn a_branch_with_no_known_main_needs_the_dev_install() {
    let dir = tempfile::tempdir().unwrap();
    let work = repository(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    git(&work, &["update-ref", "-d", "refs/remotes/origin/main"]);

    let output = skippable(dir.path());

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "no merge base with origin/main\n");
}

#[test]
fn ship_skips_the_dev_install_and_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("README.md"), "changed\n").unwrap();

    let output = ship(dir.path(), "docs: readme");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        ["cargo fmt", "gates.sh ", "gh pr view topic --json number"]
    );
    assert!(
        stdout(&output).contains(
            "skipping dev-install: nothing on this branch feeds the binary, and conveyor-dev is another worktree's build\n"
        ),
        "{}",
        stdout(&output)
    );
    assert_eq!(pushed_subject(dir.path()), "docs: readme");
}

#[test]
fn ship_runs_the_dev_install_when_the_commit_feeds_the_binary() {
    let dir = tempfile::tempdir().unwrap();
    let work = ship_sandbox(dir.path());
    link_conveyor_dev(dir.path(), &another_build(dir.path()));
    fs::write(work.join("src/main.rs"), "fn main() { }\n").unwrap();

    let output = ship(dir.path(), "fix: main");

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        calls(dir.path()),
        [
            "cargo fmt",
            "gates.sh ",
            "dev-install.sh ",
            "gh pr view topic --json number",
        ]
    );
    assert!(
        !stdout(&output).contains("skipping dev-install"),
        "{}",
        stdout(&output)
    );
    assert_eq!(pushed_subject(dir.path()), "fix: main");
}

fn gates_sandbox(root: &Path) -> PathBuf {
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    symlink(which("bash"), bin.join("bash")).unwrap();
    symlink(which("dirname"), bin.join("dirname")).unwrap();
    symlink(recorder(), bin.join("cargo")).unwrap();
    link_script(root, "gates.sh");
    symlink(recorder(), root.join("scripts/scrub-check.sh")).unwrap();
    bin
}

fn gates(root: &Path) -> Output {
    Command::new(root.join("scripts/gates.sh"))
        .env_clear()
        .env("PATH", root.join("bin"))
        .env("CALLS_LOG", root.join("calls.log"))
        .output()
        .unwrap()
}

fn calls(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn link_script(root: &Path, name: &str) {
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name);
    symlink(source, scripts.join(name)).unwrap();
}

fn recorder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/record-call.sh")
}

fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} is not on PATH"))
}

fn repository(root: &Path) -> PathBuf {
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

fn ship_sandbox(root: &Path) -> PathBuf {
    let work = repository(root);
    git(root, &["init", "-q", "--bare", "origin.git"]);
    git(&work, &["remote", "add", "origin", "../origin.git"]);
    link_script(&work, "ship.sh");
    symlink(recorder(), work.join("scripts/gates.sh")).unwrap();
    symlink(recorder(), work.join("scripts/dev-install.sh")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    symlink(recorder(), root.join("bin/cargo")).unwrap();
    symlink(recorder(), root.join("bin/gh")).unwrap();
    work
}

fn ship(root: &Path, message: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(root.join("work/scripts/ship.sh")))
        .arg(message)
        .env("HOME", root.join("home"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .output()
        .unwrap()
}

fn pushed_subject(root: &Path) -> String {
    let output = isolated(
        Command::new("git")
            .current_dir(root.join("origin.git"))
            .args(["log", "-1", "--format=%s", "topic"]),
    )
    .output()
    .unwrap();
    stdout(&output).trim().to_string()
}

fn git(dir: &Path, args: &[&str]) {
    let status = isolated(Command::new("git").current_dir(dir).args(args))
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn isolated(command: &mut Command) -> &mut Command {
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
}

fn another_build(root: &Path) -> PathBuf {
    build_in(&root.join("elsewhere"))
}

fn build_in(worktree: &Path) -> PathBuf {
    let binary = worktree.join("target/release/conveyor");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    symlink(which("true"), &binary).unwrap();
    binary
}

fn link_conveyor_dev(root: &Path, target: &Path) {
    let bin = root.join("home/.local/bin");
    fs::create_dir_all(&bin).unwrap();
    symlink(target, bin.join("conveyor-dev")).unwrap();
}

fn skippable(root: &Path) -> Output {
    isolated(Command::new(which("bash")).arg(root.join("work/scripts/dev-install-skippable.sh")))
        .env("HOME", root.join("home"))
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
