use std::fs;
use std::os::unix::fs::PermissionsExt;

use conveyor::github::{CliGh, Github};

fn shim(dir: &tempfile::TempDir, script: &str) -> CliGh {
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(&gh, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    CliGh::with_program(gh)
}

#[test]
fn graphql_argv_passes_the_query_file_and_typed_variables() {
    let args = CliGh::graphql_args(
        "query($q: String!, $first: Int!) { x }",
        &[("q", "is:pr".into()), ("first", 20u64.into())],
    );
    assert_eq!(
        args,
        vec![
            "api",
            "graphql",
            "-f",
            "query=query($q: String!, $first: Int!) { x }",
            "-f",
            "q=is:pr",
            "-F",
            "first=20",
        ]
    );
}

#[test]
fn graphql_returns_the_parsed_json_from_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, r#"echo '{"data":{"ok":true}}'"#);

    let value = gh.graphql("query { x }", &[]).unwrap();

    assert_eq!(
        value.pointer("/data/ok"),
        Some(&serde_json::Value::Bool(true))
    );
}

#[test]
fn a_failing_gh_reports_its_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo 'gh: HTTP 401: Bad credentials' >&2; exit 1");

    let err = gh.graphql("query { x }", &[]).unwrap_err();

    assert_eq!(err.to_string(), "gh: HTTP 401: Bad credentials");
}

#[test]
fn a_missing_gh_binary_says_how_to_install_it() {
    let gh = CliGh::with_program("/nonexistent/gh".into());

    let err = gh.graphql("query { x }", &[]).unwrap_err();

    assert_eq!(
        err.to_string(),
        "gh not found on PATH; install gh (brew install gh) and run gh auth login"
    );
}
