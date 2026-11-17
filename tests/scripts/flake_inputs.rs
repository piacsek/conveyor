use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;

use crate::helpers::{piped, piped_on, stderr, stdout};

#[test]
fn every_path_the_flake_reads_is_an_input() {
    let probes = probes(&paths_named_in(&read("flake.nix")).unwrap());

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
        "README.md\0docs/details.png\0tests/cli.rs\0.github/workflows/release.yml\0src.md\0scripts/ship.sh\0",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_grep_that_cannot_run_fails_the_filter() {
    let dir = tempfile::tempdir().unwrap();
    let grep = dir.path().join("grep");
    fs::write(&grep, "#!/bin/sh\nexit 2\n").unwrap();
    fs::set_permissions(&grep, fs::Permissions::from_mode(0o755)).unwrap();
    let search_path = std::env::join_paths(
        std::iter::once(dir.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();

    let output = piped_on(&search_path, "flake-inputs.sh", "src/main.rs\0");

    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn a_path_named_in_the_flake_is_read_from_its_path_literals() {
    assert_eq!(
        paths_named_in(
            "root = ./.;\nfileset = unions [ ./Cargo.toml ./src ];\nreadFile ./a.toml);"
        ),
        Ok(vec![
            "Cargo.toml".to_string(),
            "src".to_string(),
            "a.toml".to_string()
        ])
    );
}

#[test]
fn the_flake_as_it_stands_names_only_paths_it_reads() {
    assert!(paths_named_in(&read("flake.nix")).is_ok());
}

#[test]
fn the_whole_tree_outside_the_fileset_root_is_refused() {
    let flake = mutated("./src\n", "(pkgs.lib.fileset.gitTracked ./.)\n");

    assert!(paths_named_in(&flake).is_err());
}

#[test]
fn a_second_root_of_the_whole_tree_is_refused() {
    let flake = mutated("./src\n", "./src\n./.\n");

    assert!(paths_named_in(&flake).is_err());
}

#[test]
fn a_path_appended_to_the_whole_tree_is_refused() {
    let flake = mutated("./CHANGELOG.md\n", "(./. + \"/assets\")\n");

    assert!(paths_named_in(&flake).is_err());
}

#[test]
fn a_path_appended_to_a_directory_is_refused() {
    let flake = mutated("./CHANGELOG.md\n", "(./docs + \"/assets\")\n");

    assert!(paths_named_in(&flake).is_err());
}

fn mutated(from: &str, to: &str) -> String {
    let flake = read("flake.nix");
    assert!(flake.contains(from), "flake.nix no longer holds {from:?}");
    flake.replacen(from, to, 1)
}

fn dropped(paths: &[String]) -> Vec<String> {
    let output = flake_inputs(&format!("{}\0", paths.join("\0")));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let printed = stdout(&output);
    paths
        .iter()
        .filter(|path| !printed.split('\0').any(|line| line == path.as_str()))
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

fn paths_named_in(text: &str) -> Result<Vec<String>, String> {
    let outside_the_root = text.replacen("root = ./.;", "", 1);
    if outside_the_root
        .split_whitespace()
        .collect::<String>()
        .contains("+\"/")
    {
        return Err("a path built with + \"/…\"".to_string());
    }
    outside_the_root
        .split(|c: char| c.is_whitespace() || "()[];".contains(c))
        .filter_map(|token| token.strip_prefix("./"))
        .map(|path| match path {
            "" | "." => Err(format!("./{path} outside the root of the fileset")),
            _ => Ok(path.to_string()),
        })
        .collect()
}

fn read(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

fn flake_inputs(paths: &str) -> Output {
    piped("flake-inputs.sh", paths)
}
