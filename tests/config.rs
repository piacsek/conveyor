use conveyor::config::{Config, load};

fn write(dir: &tempfile::TempDir, text: &str) -> std::path::PathBuf {
    let path = dir.path().join("config.toml");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_missing_file_yields_the_defaults() {
    let dir = tempfile::tempdir().unwrap();

    let config = load(&dir.path().join("config.toml")).unwrap();

    assert_eq!(config, Config::default());
    assert_eq!(config.prs.limit, 20);
    assert_eq!(config.prs.refresh_secs, 60);
}

#[test]
fn a_partial_file_overrides_only_the_keys_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "[prs]\nquery = \"is:pr involves:@me\"\n");

    let config = load(&path).unwrap();

    assert_eq!(config.prs.query, "is:pr involves:@me");
    assert_eq!(config.prs.limit, 20);
}

#[test]
fn an_unknown_key_or_bad_toml_fails_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();

    let err = load(&write(&dir, "[prs]\nquerry = \"x\"\n")).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("config.toml"), "{text}");
    assert!(text.contains("querry"), "{text}");

    let err = load(&write(&dir, "not toml at all [[[")).unwrap_err();
    assert!(err.to_string().contains("config.toml"), "{err}");
}

#[test]
fn the_effective_config_prints_as_toml_that_loads_back_identically() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.prs.limit = 5;

    let text = config.to_toml();
    let reloaded = load(&write(&dir, &text)).unwrap();

    assert_eq!(reloaded, config);
    assert!(text.contains("[prs]"), "{text}");
    assert!(text.contains("limit = 5"), "{text}");
}

#[test]
fn the_config_path_prefers_the_override_then_xdg_then_home() {
    use conveyor::config::path;
    use std::path::{Path, PathBuf};

    let home = Path::new("/home/me");
    assert_eq!(
        path(
            Some(PathBuf::from("/x/c.toml")),
            Some(PathBuf::from("/xdg")),
            home
        ),
        PathBuf::from("/x/c.toml")
    );
    assert_eq!(
        path(None, Some(PathBuf::from("/xdg")), home),
        PathBuf::from("/xdg/conveyor/config.toml")
    );
    assert_eq!(
        path(None, None, home),
        PathBuf::from("/home/me/.config/conveyor/config.toml")
    );
}
