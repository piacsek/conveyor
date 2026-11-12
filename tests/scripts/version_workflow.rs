use std::fs;
use std::path::Path;

#[test]
fn the_version_check_runs_the_copy_on_main_for_every_pull_request_into_main_or_retargeted_to_it() {
    let workflow = workflow();

    assert!(workflow.contains("on:\n  pull_request_target:\n    types: [opened, synchronize, reopened, edited]\n    branches: [main]\n"));
    assert!(workflow.contains("jobs:\n  version:\n"));
}

#[test]
fn the_version_check_holds_a_read_only_token_and_no_secret() {
    let workflow = workflow();

    assert!(workflow.contains("permissions:\n  contents: read\n"));
    assert!(!workflow.contains("write"));
    assert!(!workflow.contains("secrets."));
}

#[test]
fn the_token_reaches_only_the_fetch_and_is_never_left_in_the_checkout() {
    let workflow = workflow();
    let (before_check, check) = workflow
        .split_once("- name: Only the release workflow may set the version")
        .unwrap();

    assert!(workflow.contains("persist-credentials: false\n"));
    assert_eq!(workflow.matches("github.token").count(), 1);
    assert!(before_check.contains("TOKEN: ${{ github.token }}\n"));
    assert!(before_check.contains("GIT_CONFIG_KEY_0=http.https://github.com/.extraheader"));
    assert!(!check.to_lowercase().contains("token"));
}

#[test]
fn the_version_check_never_checks_out_the_pull_request() {
    let workflow = workflow();

    assert!(!workflow.contains("ref:"));
    assert!(!workflow.contains("head.ref"));
    assert!(!workflow.contains("github.head_ref"));
    assert!(workflow.contains("scripts/version-check.sh HEAD \"$HEAD_SHA\" 2>&1 || status=$?\n"));
}

#[test]
fn the_check_runs_with_workflow_commands_stopped_by_a_token_made_at_run_time() {
    let workflow = workflow();
    let (_, check) = workflow
        .split_once("- name: Only the release workflow may set the version")
        .unwrap();
    let (before_stop, stopped) = check
        .split_once("echo \"::stop-commands::$stop\"\n")
        .unwrap();
    let (during, after) = stopped.split_once("echo \"::$stop::\"\n").unwrap();

    assert!(before_stop.contains("stop=\"$(od -An -N16 -tx1 /dev/urandom | tr -d ' \\n')\"\n"));
    assert!(!before_stop.contains("scripts/version-check.sh"));
    assert!(during.contains("scripts/version-check.sh HEAD \"$HEAD_SHA\" 2>&1 || status=$?\n"));
    assert!(after.contains("exit \"$status\"\n"));
    assert!(!after.contains("scripts/version-check.sh"));
    assert_eq!(workflow.matches("::stop-commands::").count(), 1);
}

#[test]
fn the_version_job_gives_up_after_five_minutes() {
    let workflow = workflow();

    assert!(
        workflow
            .contains("jobs:\n  version:\n    runs-on: ubuntu-latest\n    timeout-minutes: 5\n")
    );
}

#[test]
fn every_action_of_the_version_check_is_pinned_to_a_commit() {
    let workflow = workflow();

    assert!(
        workflow
            .lines()
            .filter_map(|line| line.trim().strip_prefix("- uses: "))
            .all(pinned),
        "{workflow}"
    );
}

fn workflow() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/version.yml"))
        .unwrap()
}

fn pinned(action: &str) -> bool {
    let sha = action
        .split_once('@')
        .map(|(_, rest)| rest.split(' ').next().unwrap_or(""))
        .unwrap_or("");
    sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) && action.contains(" # v")
}
