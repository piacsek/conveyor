use conveyor::config::{Config, load};

#[test]
fn the_defaults_pass_their_own_checks() {
    let dir = tempfile::tempdir().unwrap();

    let loaded = load(&write(&dir, &Config::default().to_toml()));

    assert_eq!(loaded.unwrap(), Config::default());
}

#[test]
fn the_demo_config_passes_the_checks() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sources/demo/config.toml");

    let loaded = load(&path);

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn a_refresh_of_ten_seconds_loads_everywhere() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[prs]\nrefresh_secs = 10\n[[repo]]\nname = \"acme/webapp\"\nrefresh_secs = 10\n[[repo.deploy]]\nrefresh_secs = 10\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn the_ends_of_the_page_range_load() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[prs]\nlimit = 1\n[[prs.tab]]\nname = \"mine\"\nquery = \"is:pr\"\nlimit = 100\n[[repo]]\nname = \"acme/webapp\"\nbuilds = 100\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn the_ends_of_the_layout_ranges_load() {
    let dir = tempfile::tempdir().unwrap();
    let narrow = "[ui]\ndetails_percent = 10\nmin_column_width = 0\n";
    let wide = "[ui]\ndetails_percent = 90\nmin_column_width = 1000\n";

    let narrow = load(&write(&dir, narrow));
    let wide = load(&write(&dir, wide));

    assert!(narrow.is_ok(), "{narrow:?}");
    assert!(wide.is_ok(), "{wide:?}");
}

#[test]
fn a_repository_name_needs_one_owner_and_one_name() {
    assert!(refused_repo("acme/").contains("[[repo]] name \"acme/\" is not owner/name"));
    assert!(refused_repo("/webapp").contains("[[repo]] name \"/webapp\" is not owner/name"));
    assert!(refused_repo("acme/web/app").contains("is not owner/name"));
    assert!(refused_repo("acme/web app").contains("is not owner/name"));
    assert!(refused_repo("").contains("[[repo]] name \"\" is not owner/name"));
}

#[test]
fn an_environment_needs_every_field() {
    let full = "name = \"prod\"\ncontext = \"ctx\"\nnamespace = \"api\"\ndeployment = \"api\"\n";

    assert!(
        refused_env(&full.replace("prod", " "))
            .contains("a [[repo.deploy.env]] of acme/webapp has a blank name")
    );
    assert!(
        refused_env(&full.replace("\"ctx\"", "\"\""))
            .contains("\"prod\" of acme/webapp has a blank context")
    );
    assert!(
        refused_env(&full.replace("namespace = \"api\"", ""))
            .contains("\"prod\" of acme/webapp has a blank namespace")
    );
    assert!(
        refused_env(&full.replace("deployment = \"api\"", "deployment = \" \""))
            .contains("\"prod\" of acme/webapp has a blank deployment")
    );
}

fn refused_env(env: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let text =
        format!("[[repo]]\nname = \"acme/webapp\"\n[[repo.deploy]]\n[[repo.deploy.env]]\n{env}");
    load(&write(&dir, &text)).unwrap_err().to_string()
}

fn refused_repo(name: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let text = format!("[[repo]]\nname = \"{name}\"\n");
    load(&write(&dir, &text)).unwrap_err().to_string()
}

fn write(dir: &tempfile::TempDir, text: &str) -> std::path::PathBuf {
    let path = dir.path().join("config.toml");
    std::fs::write(&path, text).unwrap();
    path
}
