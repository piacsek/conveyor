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
