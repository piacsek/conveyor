use conveyor::config::{Config, Key, MergeMethod, UpdateMethod, load};

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

#[test]
fn repo_tables_carry_defaults_for_the_build_workflow_and_intervals() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "[[repo]]\nname = \"acme/webapp\"\n\n[[repo]]\nname = \"acme/auth\"\nmain_workflow = \"build.yml\"\nbuilds = 5\nrefresh_secs = 15\n",
    );

    let config = load(&path).unwrap();

    assert_eq!(config.repo.len(), 2);
    assert_eq!(config.repo[0].name, "acme/webapp");
    assert_eq!(config.repo[0].main_workflow, "CI/CD");
    assert_eq!(config.repo[0].builds, 10);
    assert_eq!(config.repo[0].refresh_secs, 30);
    assert_eq!(config.repo[1].main_workflow, "build.yml");
    assert_eq!(
        (config.repo[1].builds, config.repo[1].refresh_secs),
        (5, 15)
    );
    assert!(Config::default().repo.is_empty());
    assert!(
        config.to_toml().contains("[[repo]]"),
        "{}",
        config.to_toml()
    );
}

#[test]
fn deploy_tables_describe_systems_and_their_environments() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        r#"[[repo]]
name = "acme/webapp"

[[repo.deploy]]
system = "api"
refresh_secs = 60

[[repo.deploy.env]]
name = "staging"
fetcher = "kubectl"
context = "teleport-staging"
namespace = "api"
deployment = "api"

[[repo.deploy.env]]
name = "prod"
context = "teleport-prod"
namespace = "api"
deployment = "api"
"#,
    );

    let config = load(&path).unwrap();

    let deploy = &config.repo[0].deploy;
    assert_eq!(deploy.len(), 1);
    assert_eq!(deploy[0].system, "api");
    assert_eq!(deploy[0].refresh_secs, 60);
    assert_eq!(deploy[0].env.len(), 2);
    assert_eq!(deploy[0].env[1].name, "prod");
    assert_eq!(deploy[0].env[1].fetcher, conveyor::config::Fetcher::Kubectl);
    assert_eq!(deploy[0].env[1].context, "teleport-prod");
    assert_eq!(deploy[0].env[1].deployment, "api");
    assert!(
        config.to_toml().contains("[[repo.deploy.env]]"),
        "{}",
        config.to_toml()
    );

    let err = load(&write(&dir, "[[repo]]\nname = \"a/b\"\n[[repo.deploy]]\nsystem = \"x\"\n[[repo.deploy.env]]\nname = \"p\"\nfetcher = \"argocd\"\n")).unwrap_err();
    assert!(err.to_string().contains("argocd"), "{err}");
}

#[test]
fn the_merge_method_defaults_to_squash_and_accepts_the_other_two() {
    let dir = tempfile::tempdir().unwrap();

    let defaults = load(&dir.path().join("config.toml")).unwrap();
    let rebase = load(&write(&dir, "[prs]\nmerge_method = \"rebase\"\n")).unwrap();
    let merge = load(&write(&dir, "[prs]\nmerge_method = \"merge\"\n")).unwrap();

    assert_eq!(defaults.prs.merge_method, MergeMethod::Squash);
    assert_eq!(rebase.prs.merge_method, MergeMethod::Rebase);
    assert_eq!(merge.prs.merge_method, MergeMethod::Merge);
}

#[test]
fn an_unknown_merge_method_fails_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();

    let err = load(&write(&dir, "[prs]\nmerge_method = \"fast\"\n")).unwrap_err();

    assert!(err.to_string().contains("config.toml"), "{err}");
    assert!(err.to_string().contains("fast"), "{err}");
}

#[test]
fn the_update_method_defaults_to_merge_and_accepts_rebase() {
    let dir = tempfile::tempdir().unwrap();

    let defaults = load(&dir.path().join("config.toml")).unwrap();
    let rebase = load(&write(&dir, "[prs]\nupdate_method = \"rebase\"\n")).unwrap();

    assert_eq!(defaults.prs.update_method, UpdateMethod::Merge);
    assert_eq!(rebase.prs.update_method, UpdateMethod::Rebase);
}

#[test]
fn squash_is_not_an_update_method() {
    let dir = tempfile::tempdir().unwrap();

    let err = load(&write(&dir, "[prs]\nupdate_method = \"squash\"\n")).unwrap_err();

    assert!(err.to_string().contains("config.toml"), "{err}");
    assert!(err.to_string().contains("squash"), "{err}");
}

#[test]
fn a_repo_takes_an_optional_path_that_stays_out_of_the_printed_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "[[repo]]\nname = \"acme/webapp\"\npath = \"~/projects/webapp\"\n",
    );

    let config = load(&path).unwrap();

    assert_eq!(
        config.repo[0].path,
        Some(std::path::PathBuf::from("~/projects/webapp"))
    );
    assert_eq!(conveyor::config::Repo::default().path, None);
    let unset = Config {
        repo: vec![conveyor::config::Repo::default()],
        ..Config::default()
    };
    assert!(!unset.to_toml().contains("path"));
    assert!(config.to_toml().contains("path = \"~/projects/webapp\""));
}

#[test]
fn a_leading_tilde_in_a_path_is_the_home_directory() {
    use conveyor::config::expand_home;
    use std::path::{Path, PathBuf};
    let home = Path::new("/home/dev");

    assert_eq!(
        expand_home(Path::new("~/projects/webapp"), Some(home)),
        PathBuf::from("/home/dev/projects/webapp")
    );
    assert_eq!(
        expand_home(Path::new("~"), Some(home)),
        PathBuf::from("/home/dev")
    );
    assert_eq!(
        expand_home(Path::new("/srv/webapp"), Some(home)),
        PathBuf::from("/srv/webapp")
    );
    assert_eq!(
        expand_home(Path::new("~other/webapp"), Some(home)),
        PathBuf::from("~other/webapp")
    );
    assert_eq!(
        expand_home(Path::new("~/webapp"), None),
        PathBuf::from("~/webapp")
    );
}

#[test]
fn tab_names_the_tab_key() {
    assert_eq!(Key::parse("tab"), Ok(Key::Tab));
}

#[test]
fn shift_tab_names_the_back_tab_key() {
    assert_eq!(Key::parse("shift-tab"), Ok(Key::BackTab));
}

#[test]
fn a_single_character_names_that_character() {
    assert_eq!(Key::parse("]"), Ok(Key::Char(']')));
}

#[test]
fn ctrl_and_a_character_names_that_chord() {
    assert_eq!(Key::parse("ctrl-n"), Ok(Key::Ctrl('n')));
}

#[test]
fn an_unknown_key_name_is_refused() {
    let err = Key::parse("bogus").unwrap_err();

    assert!(err.contains("\"bogus\""), "{err}");
}

#[test]
fn a_key_conveyor_already_uses_is_refused_naming_what_it_does() {
    let err = Key::parse("j").unwrap_err();

    assert!(err.contains("move"), "{err}");
}

#[test]
fn a_digit_is_refused_since_it_picks_a_tab() {
    let err = Key::parse("3").unwrap_err();

    assert!(err.contains("tab"), "{err}");
}

#[test]
fn a_control_chord_conveyor_already_uses_is_refused() {
    let err = Key::parse("ctrl-d").unwrap_err();

    assert!(err.contains("details"), "{err}");
}

#[test]
fn the_tab_keys_default_to_tab_and_shift_tab() {
    let config = Config::default();

    assert_eq!(config.ui.keys.next_tab, Key::Tab);
    assert_eq!(config.ui.keys.previous_tab, Key::BackTab);
}

#[test]
fn the_tab_keys_read_from_ui_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        &dir,
        "[ui.keys]\nnext_tab = \"]\"\nprevious_tab = \"ctrl-p\"\n",
    );

    let config = load(&path).unwrap();

    assert_eq!(config.ui.keys.next_tab, Key::Char(']'));
    assert_eq!(config.ui.keys.previous_tab, Key::Ctrl('p'));
}

#[test]
fn a_key_already_in_use_fails_naming_the_file_and_the_meaning() {
    let dir = tempfile::tempdir().unwrap();

    let err = load(&write(&dir, "[ui.keys]\nnext_tab = \"j\"\n")).unwrap_err();

    let text = err.to_string();
    assert!(text.contains("config.toml"), "{text}");
    assert!(text.contains("move"), "{text}");
}

#[test]
fn the_same_key_for_both_tab_keys_fails() {
    let dir = tempfile::tempdir().unwrap();

    let err = load(&write(&dir, "[ui.keys]\nprevious_tab = \"tab\"\n")).unwrap_err();

    let text = err.to_string();
    assert!(text.contains("next_tab"), "{text}");
}

#[test]
fn the_printed_tab_keys_load_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.ui.keys.next_tab = Key::Ctrl('n');

    let reloaded = load(&write(&dir, &config.to_toml())).unwrap();

    assert_eq!(reloaded, config);
}
