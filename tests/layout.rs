use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn every_path_the_layout_names_exists() {
    let agents = read(&repo().join("AGENTS.md"));
    let missing: Vec<String> = layout_paths(&agents)
        .into_iter()
        .filter(|path| !repo().join(path).exists())
        .collect();
    assert!(
        missing.is_empty(),
        "the `## Layout` of AGENTS.md names paths that do not exist: {missing:?}"
    );
}

#[test]
fn the_layout_names_paths() {
    let agents = read(&repo().join("AGENTS.md"));
    assert!(layout_paths(&agents).len() > 10);
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn layout_paths(agents: &str) -> Vec<String> {
    agents
        .lines()
        .skip_while(|line| line.trim_end() != "## Layout")
        .skip_while(|line| !line.starts_with("```"))
        .skip(1)
        .take_while(|line| !line.starts_with("```"))
        .flat_map(str::split_whitespace)
        .map(|word| word.trim_matches(|c: char| "`,;:()".contains(c)))
        .filter(|word| names_a_path(word))
        .map(str::to_string)
        .collect()
}

fn names_a_path(word: &str) -> bool {
    let extensions = [
        ".rs", ".sh", ".md", ".toml", ".lock", ".nix", ".json", ".yml",
    ];
    word.contains('/') || extensions.iter().any(|ext| word.ends_with(ext))
}
