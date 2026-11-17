use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

use crate::helpers::{calls, isolated, link_script, stderr, stdout, which};

const METRICS: &str = "# Time and tokens per phase

| Phase | PR | Start | Ready | Rounds | Notes |
| --- | --- | --- | --- | --- | --- |
| 39 | #1 | 2026-11-01 01:00Z | 01:30Z | 1 | old |
| 41 | #2 | 2026-11-02 01:00Z | 01:40Z | 3 | |
| 42 | #3 | 2026-11-03 01:00Z | | 2 | shared session |
";

#[test]
fn the_report_counts_the_hurdles_by_tag_from_phase_40_when_no_sweep_ran_yet() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 39, "- `ci` too old to count → none\n");
    plan(
        dir.path(),
        41,
        "- `ci` the runner image moved → code\n- `review-loop` round 2 found a hole → none\n",
    );
    plan(dir.path(), 42, "- `ci` a cache went stale → none\n");

    let output = report(dir.path(), &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.starts_with("# Sweep report: phases 40 to 42\n"),
        "{text}"
    );
    assert!(
        text.contains("| Tag | Hurdles |\n| --- | --- |\n| `ci` | 2 |\n| `review-loop` | 1 |\n"),
        "{text}"
    );
    assert!(!text.contains("too old to count"), "{text}");
}

#[test]
fn the_report_lists_every_hurdle_that_went_nowhere_with_its_phase() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(
        dir.path(),
        41,
        "- `ci` the runner image moved → code\n- `review-loop` round 2 found\n  a hole → none\n",
    );
    plan(dir.path(), 42, "- `ci` a cache went stale → none\n");

    let text = stdout(&report(dir.path(), &[]));

    assert!(
        text.contains(
            "## Went nowhere (→ none)\n\n- 41 `review-loop` round 2 found a hole\n- 42 `ci` a cache went stale\n"
        ),
        "{text}"
    );
}

#[test]
fn the_report_groups_every_hurdle_under_its_tag() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 41, "- `ci` the runner image moved → code\n");
    plan(dir.path(), 42, "- `ci` a cache went stale → none\n");

    let text = stdout(&report(dir.path(), &[]));

    assert!(
        text.contains(
            "### `ci`\n\n- 41 the runner image moved → code\n- 42 a cache went stale → none\n"
        ),
        "{text}"
    );
}

#[test]
fn the_report_starts_after_the_last_phase_that_holds_a_sweep() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 41, "- `ci` before the sweep → none\n");
    fs::write(
        dir.path().join("work/plans/phase-45-sweep.md"),
        "# Phase 45\n\n## Outcome\n\n### Sweep\n\nDone.\n\n## Hurdles\n\n- none recorded\n",
    )
    .unwrap();
    plan(dir.path(), 46, "- `tooling` after the sweep → none\n");

    let text = stdout(&report(dir.path(), &[]));

    assert!(
        text.starts_with("# Sweep report: phases 46 to 46\n"),
        "{text}"
    );
    assert!(text.contains("after the sweep"), "{text}");
    assert!(!text.contains("before the sweep"), "{text}");
}

#[test]
fn the_report_starts_where_it_is_told_to() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 41, "- `ci` left out → none\n");
    plan(dir.path(), 42, "- `ci` kept → none\n");

    let text = stdout(&report(dir.path(), &["42"]));

    assert!(
        text.starts_with("# Sweep report: phases 42 to 42\n"),
        "{text}"
    );
    assert!(!text.contains("left out"), "{text}");
}

#[test]
fn the_report_carries_the_metrics_rows_of_its_phases_and_names_their_empty_cells() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 41, "- none recorded\n");
    plan(dir.path(), 42, "- none recorded\n");

    let text = stdout(&report(dir.path(), &[]));

    assert!(
        text.contains(
            "## Metrics\n\n| Phase | PR | Start | Ready | Rounds | Notes |\n| --- | --- | --- | --- | --- | --- |\n| 41 | #2 |"
        ),
        "{text}"
    );
    assert!(!text.contains("| 39 | #1 |"), "{text}");
    assert!(
        text.contains("## Empty cells\n\n- 41: Notes\n- 42: Ready\n"),
        "{text}"
    );
}

#[test]
fn a_phase_with_no_hurdles_section_is_named_in_the_report() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    fs::write(
        dir.path().join("work/plans/phase-41-x.md"),
        "# Phase 41\n\n## Outcome\n\nShipped.\n",
    )
    .unwrap();

    let text = stdout(&report(dir.path(), &[]));

    assert!(text.contains("## No Hurdles section\n\n- 41\n"), "{text}");
}

#[test]
fn no_sweep_is_due_while_the_next_phase_is_not_a_multiple_of_five() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 57, "- none recorded\n");

    let output = issue(dir.path(), "no call fails");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "no sweep is due: the next phase is 58\n");
    assert!(calls(dir.path()).is_empty(), "{:?}", calls(dir.path()));
}

#[test]
fn a_due_sweep_opens_its_issue_with_the_report_as_the_body() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 59, "- `ci` stale → none\n");

    let output = issue(dir.path(), "no call fails");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let calls = calls(dir.path());
    assert_eq!(
        calls[0],
        "gh issue list --state all --search \"Sweep due before phase 60\" in:title --json title --jq .[].title"
    );
    assert!(
        calls[1].starts_with("gh issue create --title Sweep due before phase 60 --body-file "),
        "{calls:?}"
    );
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(
        stdout(&output),
        "opened the issue: Sweep due before phase 60\n"
    );
}

#[test]
fn a_due_sweep_whose_issue_exists_opens_no_second_one() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 59, "- none recorded\n");
    fs::write(
        dir.path().join("replies/list"),
        "Sweep due before phase 60\n",
    )
    .unwrap();

    let output = issue(dir.path(), "no call fails");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(calls(dir.path()).len(), 1, "{:?}", calls(dir.path()));
    assert_eq!(
        stdout(&output),
        "the issue already exists: Sweep due before phase 60\n"
    );
}

#[test]
fn an_issue_whose_title_only_starts_the_same_does_not_count_as_the_sweep() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 59, "- none recorded\n");
    fs::write(
        dir.path().join("replies/list"),
        "Sweep due before phase 600\n",
    )
    .unwrap();

    issue(dir.path(), "no call fails");

    assert_eq!(calls(dir.path()).len(), 2, "{:?}", calls(dir.path()));
}

#[test]
fn issues_that_cannot_be_read_stop_the_job() {
    let dir = tempfile::tempdir().unwrap();
    sandbox(dir.path());
    plan(dir.path(), 59, "- none recorded\n");

    let output = issue(dir.path(), "issue list");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "the sweep issues could not be read: HTTP 502\n"
    );
    assert_eq!(calls(dir.path()).len(), 1, "{:?}", calls(dir.path()));
}

#[test]
fn every_push_to_main_that_touches_a_plan_asks_whether_a_sweep_is_due() {
    let workflow = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/sweep.yml"),
    )
    .unwrap();

    assert_eq!(workflow, SWEEP_WORKFLOW);
}

const SWEEP_WORKFLOW: &str = "name: sweep

on:
  push:
    branches: [main]
    paths: ['plans/**']

permissions:
  contents: read

concurrency:
  group: sweep
  cancel-in-progress: false

jobs:
  issue:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      issues: write
    steps:
      - uses: actions/checkout@fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09 # v5
      - env:
          GH_TOKEN: ${{ github.token }}
        run: scripts/sweep-issue.sh
";

fn plan(root: &Path, phase: u32, hurdles: &str) {
    fs::write(
        root.join(format!("work/plans/phase-{phase}-x.md")),
        format!("# Phase {phase}\n\n## Outcome\n\nShipped.\n\n## Hurdles\n\n{hurdles}"),
    )
    .unwrap();
}

fn sandbox(root: &Path) {
    let work = root.join("work");
    link_script(&work, "sweep-report.sh");
    link_script(&work, "sweep-issue.sh");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("replies")).unwrap();
    fs::create_dir_all(work.join("plans")).unwrap();
    fs::write(work.join("plans/metrics.md"), METRICS).unwrap();
    symlink(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reply-call.sh"),
        root.join("bin/gh"),
    )
    .unwrap();
}

fn report(root: &Path, args: &[&str]) -> Output {
    run(root, "sweep-report.sh", args, "no call fails")
}

fn issue(root: &Path, failing_call: &str) -> Output {
    run(root, "sweep-issue.sh", &[], failing_call)
}

fn run(root: &Path, script: &str, args: &[&str], failing_call: &str) -> Output {
    let mut path = vec![root.join("bin")];
    path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    isolated(&mut Command::new(which("bash")))
        .arg(root.join("work/scripts").join(script))
        .args(args)
        .current_dir(root.join("work"))
        .env("PATH", std::env::join_paths(path).unwrap())
        .env("CALLS_LOG", root.join("calls.log"))
        .env("REPLIES", root.join("replies"))
        .env("FAILING_CALL", failing_call)
        .env("FAILING_STDERR", "HTTP 502")
        .output()
        .unwrap()
}
