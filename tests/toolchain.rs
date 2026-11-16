use std::path::Path;

#[test]
fn ci_installs_the_pinned_toolchain() {
    assert_eq!(workflow_toolchain("ci.yml"), channel());
}

#[test]
fn the_release_builds_with_the_pinned_toolchain() {
    assert_eq!(workflow_toolchain("release.yml"), channel());
}

#[test]
fn the_tag_job_keeps_the_runner_image_toolchain() {
    assert!(tag_job().contains(STABLE_FOR_THE_JOB));
}

#[test]
fn rust_version_is_the_major_minor_of_the_pinned_toolchain() {
    assert_eq!(rust_version(), major_minor(&channel()));
}

fn read(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

fn channel() -> String {
    let file: toml::Table = toml::from_str(&read("rust-toolchain.toml")).unwrap();
    file["toolchain"]["channel"].as_str().unwrap().to_string()
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

fn workflow_toolchain(name: &str) -> String {
    let workflow = read(&format!(".github/workflows/{name}"));
    let pins: Vec<&str> = workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("toolchain:"))
        .map(str::trim)
        .collect();
    assert_eq!(pins.len(), 1, "{name} names {} toolchains", pins.len());
    pins[0].to_string()
}

fn tag_job() -> String {
    let release = read(".github/workflows/release.yml");
    let start = release.find("\n  tag:\n").unwrap();
    let end = release.find("\n  build:\n").unwrap();
    release[start..end].to_string()
}

const STABLE_FOR_THE_JOB: &str = "    env:\n      RUSTUP_TOOLCHAIN: stable\n";
