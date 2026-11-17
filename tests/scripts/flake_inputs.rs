use std::path::Path;
use std::process::Output;

use crate::helpers::{piped, stderr, stdout};

#[test]
fn every_path_the_flake_reads_is_an_input() {
    let probes = probes(&paths_named_in(&read("flake.nix")));

    assert_eq!(dropped(&probes), Vec::<String>::new());
}

#[test]
fn the_flake_itself_is_an_input() {
    assert_eq!(dropped(&["flake.nix".to_string()]), Vec::<String>::new());
}

#[test]
fn the_flake_lock_is_an_input() {
    assert_eq!(dropped(&["flake.lock".to_string()]), Vec::<String>::new());
}

#[test]
fn the_workflow_that_builds_the_flake_is_an_input() {
    assert_eq!(
        dropped(&[".github/workflows/ci.yml".to_string()]),
        Vec::<String>::new()
    );
}

#[test]
fn the_scripts_that_decide_the_build_are_inputs() {
    assert_eq!(
        dropped(&[
            "scripts/flake-inputs.sh".to_string(),
            "scripts/flake-build-needed.sh".to_string(),
            "scripts/flake-ok.sh".to_string(),
        ]),
        Vec::<String>::new()
    );
}

#[test]
fn paths_the_flake_does_not_read_are_left_out() {
    let output = flake_inputs(
        "README.md\ndocs/details.png\ntests/cli.rs\n.github/workflows/release.yml\nsrc.md\nscripts/ship.sh\n",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_path_named_in_the_flake_is_read_from_its_path_literals() {
    assert_eq!(
        paths_named_in(
            "root = ./.;\nfileset = unions [ ./Cargo.toml ./src ];\nreadFile ./a.toml);"
        ),
        vec!["Cargo.toml", "src", "a.toml"]
    );
}

fn dropped(paths: &[String]) -> Vec<String> {
    let output = flake_inputs(&format!("{}\n", paths.join("\n")));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let printed = stdout(&output);
    paths
        .iter()
        .filter(|path| !printed.lines().any(|line| line == path.as_str()))
        .cloned()
        .collect()
}

fn probes(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .map(|path| {
            if Path::new(env!("CARGO_MANIFEST_DIR")).join(path).is_dir() {
                format!("{path}/main.rs")
            } else {
                path.clone()
            }
        })
        .collect()
}

fn paths_named_in(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || "()[];".contains(c))
        .filter_map(|token| token.strip_prefix("./"))
        .filter(|path| !path.is_empty() && *path != ".")
        .map(str::to_string)
        .collect()
}

fn read(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

fn flake_inputs(paths: &str) -> Output {
    piped("flake-inputs.sh", paths)
}
