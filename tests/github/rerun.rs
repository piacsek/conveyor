use conveyor::sources::github::{CliGh, Github};

use crate::shims::{serialized, shim};

#[test]
fn rerun_failed_argv_reruns_only_the_failed_jobs_of_the_run() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::rerun_failed_args("acme/webapp", 1026),
        vec!["run", "rerun", "1026", "--failed", "-R", "acme/webapp"]
    );
}

#[test]
fn rerun_failed_surfaces_what_gh_printed_on_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(
        &dir,
        "echo 'run 1026 cannot be rerun; it is in progress' >&2; exit 1",
    );
    let err = gh.rerun_failed("acme/webapp", 1026).unwrap_err();
    assert_eq!(
        err.to_string(),
        "run 1026 cannot be rerun; it is in progress"
    );
}

#[test]
fn rerun_failed_succeeds_when_gh_exits_zero() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exit 0");
    assert!(gh.rerun_failed("acme/webapp", 1026).is_ok());
}
