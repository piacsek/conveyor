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
    let wide = "[ui]\ndetails_percent = 90\nmin_column_width = 16383\n";

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
    assert!(refused_repo("acme/web\\u0007app").contains("is not owner/name"));
}

#[test]
fn an_environment_needs_a_name_and_a_deployment() {
    let full = "name = \"prod\"\ndeployment = \"api\"\n";

    assert!(
        refused_env(&full.replace("prod", " "))
            .contains("a [[repo.deploy.env]] of acme/webapp has a blank name")
    );
    assert!(
        refused_env(&full.replace("deployment = \"api\"", "deployment = \" \""))
            .contains("\"prod\" of acme/webapp has a blank deployment")
    );
}

#[test]
fn an_environment_without_context_and_namespace_loads_on_the_current_ones() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\n[[repo.deploy]]\n[[repo.deploy.env]]\nname = \"prod\"\ndeployment = \"api\"\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn a_refresh_of_a_day_loads_and_longer_is_refused() {
    let dir = tempfile::tempdir().unwrap();

    let day = load(&write(&dir, "[prs]\nrefresh_secs = 86400\n"));
    let longer = load(&write(
        &dir,
        "[[repo]]\nname = \"acme/webapp\"\nrefresh_secs = 18446744073709551615\n",
    ));

    assert!(day.is_ok(), "{day:?}");
    assert!(
        longer.unwrap_err().to_string().contains(
            "[[repo]] refresh_secs of acme/webapp is 18446744073709551615; use 10 to 86400"
        )
    );
}

#[test]
fn a_deploy_on_the_second_repository_alone_loads() {
    let dir = tempfile::tempdir().unwrap();
    let text =
        "[[repo]]\nname = \"acme/webapp\"\n[[repo]]\nname = \"acme/infra\"\n[[repo.deploy]]\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn two_repositories_differing_only_in_case_are_one() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\n[[repo]]\nname = \"Acme/WebApp\"\n";

    let err = load(&write(&dir, text)).unwrap_err().to_string();

    assert!(
        err.contains("two [[repo]] are named \"Acme/WebApp\""),
        "{err}"
    );
}

#[test]
fn every_repository_is_held_to_the_ranges() {
    let first = "[[repo]]\nname = \"acme/webapp\"\n";

    assert!(
        refused(&format!(
            "{first}[[repo]]\nname = \"acme/infra\"\nbuilds = 0\n"
        ))
        .contains("[[repo]] builds of acme/infra is 0; use 1 to 100")
    );
    assert!(
        refused(&format!(
            "{first}[[repo]]\nname = \"acme/infra\"\nrefresh_secs = 5\n"
        ))
        .contains("[[repo]] refresh_secs of acme/infra is 5; use 10 to 86400")
    );
}

fn refused(text: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    load(&write(&dir, text)).unwrap_err().to_string()
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
