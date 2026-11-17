use std::path::Path;

#[test]
fn ci_installs_the_pinned_toolchain() {
    assert_eq!(workflow_input("ci.yml", "toolchain"), channel());
}

#[test]
fn the_release_builds_with_the_pinned_toolchain() {
    assert_eq!(workflow_input("release.yml", "toolchain"), channel());
}

#[test]
fn ci_installs_the_pinned_components() {
    assert_eq!(workflow_input("ci.yml", "components"), components());
}

#[test]
fn the_release_build_installs_the_pinned_components() {
    assert_eq!(workflow_input("release.yml", "components"), components());
}

#[test]
fn the_tag_job_keeps_the_runner_image_toolchain() {
    assert!(tag_job_env().contains(&"RUSTUP_TOOLCHAIN: stable".to_string()));
}

#[test]
fn rust_version_is_the_major_minor_of_the_pinned_toolchain() {
    assert_eq!(rust_version(), major_minor(&channel()));
}

#[test]
fn the_flake_builds_with_the_toolchain_the_file_names() {
    assert!(read("flake.nix").contains("fromRustupToolchainFile ./rust-toolchain.toml"));
}

#[test]
fn the_flake_names_no_rust_version_of_its_own() {
    assert_eq!(version_literals(&read("flake.nix")), Vec::<String>::new());
}

#[test]
fn a_rust_version_other_than_the_pinned_one_counts_as_a_literal() {
    assert_eq!(
        version_literals("toolchain = rust-bin.stable.\"1.99.1\".default;"),
        vec!["1.99.1".to_string()]
    );
}

#[test]
fn a_major_minor_counts_as_a_literal() {
    assert_eq!(
        version_literals("rust-bin.stable.\"1.98\".minimal"),
        vec!["1.98".to_string()]
    );
}

#[test]
fn a_nixpkgs_release_branch_is_no_rust_version() {
    assert_eq!(
        version_literals("nixpkgs.url = \"github:NixOS/nixpkgs/nixos-26.11\";"),
        Vec::<String>::new()
    );
}

#[test]
fn the_flake_leaves_the_rust_platform_of_nixpkgs_alone() {
    assert!(!uses_nixpkgs_rust(&read("flake.nix")));
}

#[test]
fn building_with_the_rust_platform_of_nixpkgs_counts_as_using_its_rust() {
    assert!(uses_nixpkgs_rust("pkgs.rustPlatform.buildRustPackage {"));
}

#[test]
fn the_toolchain_leaves_out_the_docs_the_flake_would_unpack() {
    assert_eq!(
        toolchain_file().get("profile").and_then(|p| p.as_str()),
        Some("minimal")
    );
}

fn version_literals(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_digit() && c != '.')
        .map(|run| run.trim_matches('.'))
        .filter(|run| run.starts_with("1."))
        .filter(|run| run.split('.').count() > 1 && run.split('.').all(|part| !part.is_empty()))
        .map(str::to_string)
        .collect()
}

fn uses_nixpkgs_rust(text: &str) -> bool {
    text.contains("pkgs.rustPlatform")
}

fn read(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

fn toolchain_file() -> toml::Table {
    let file: toml::Table = toml::from_str(&read("rust-toolchain.toml")).unwrap();
    file["toolchain"].as_table().unwrap().clone()
}

fn channel() -> String {
    toolchain_file()["channel"].as_str().unwrap().to_string()
}

fn components() -> String {
    toolchain_file()["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|component| component.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(", ")
}

fn rust_version() -> String {
    let manifest: toml::Table = toml::from_str(&read("Cargo.toml")).unwrap();
    manifest["package"]["rust-version"]
        .as_str()
        .unwrap()
        .to_string()
}

fn major_minor(version: &str) -> String {
    version.splitn(3, '.').take(2).collect::<Vec<_>>().join(".")
}

fn workflow_input(name: &str, key: &str) -> String {
    let workflow = read(&format!(".github/workflows/{name}"));
    let prefix = format!("{key}:");
    let values: Vec<&str> = workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix(prefix.as_str()))
        .map(str::trim)
        .collect();
    assert_eq!(
        values.len(),
        1,
        "{name} has {} `{key}:` lines",
        values.len()
    );
    values[0].to_string()
}

fn tag_job_env() -> Vec<String> {
    let release = read(".github/workflows/release.yml");
    let start = release.find("\n  tag:\n").unwrap();
    let end = release.find("\n  build:\n").unwrap();
    release[start..end]
        .lines()
        .skip_while(|line| *line != "    env:")
        .skip(1)
        .take_while(|line| line.starts_with("      "))
        .filter_map(|line| line.strip_prefix("      "))
        .filter(|line| !line.starts_with(' '))
        .map(str::to_string)
        .collect()
}
