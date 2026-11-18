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
fn a_refresh_of_five_seconds_loads_everywhere() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[prs]\nrefresh_secs = 5\n[[repo]]\nname = \"acme/webapp\"\nrefresh_secs = 5\n[[repo.deploy]]\nrefresh_secs = 5\n[[repo.deploy.env]]\nname = \"prod\"\ndeployment = \"api\"\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn one_repository_listed_twice_may_set_its_merge_method_once() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\n[[repo]]\nname = \"acme/webapp\"\nbranch = \"develop\"\nmerge_method = \"merge\"\n";

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
    let narrow = "[ui]\nmin_column_width = 0\n";
    let wide = "[ui]\nmin_column_width = 16383\n";

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
fn a_refresh_of_any_length_from_five_seconds_loads() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\nrefresh_secs = 18446744073709551615\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn the_edges_of_the_details_pane_load() {
    let dir = tempfile::tempdir().unwrap();

    let small = load(&write(&dir, "[ui]\ndetails_percent = 1\n"));
    let large = load(&write(&dir, "[ui]\ndetails_percent = 99\n"));

    assert!(small.is_ok(), "{small:?}");
    assert!(large.is_ok(), "{large:?}");
}

#[test]
fn a_details_pane_of_nothing_or_of_everything_is_refused() {
    assert!(
        refused("[ui]\ndetails_percent = 0\n").contains("[ui] details_percent is 0; use 1 to 99")
    );
    assert!(
        refused("[ui]\ndetails_percent = 100\n")
            .contains("[ui] details_percent is 100; use 1 to 99")
    );
}

#[test]
fn a_page_github_would_not_answer_as_asked_is_refused() {
    assert!(refused("[prs]\nlimit = 0\n").contains("[prs] limit is 0; use 1 to 100"));
    assert!(
        refused("[[repo]]\nname = \"acme/webapp\"\nbuilds = 101\n")
            .contains("[[repo]] builds of acme/webapp is 101; use 1 to 100")
    );
}

#[test]
fn one_repository_on_two_branches_loads() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\nbranch = \"main\"\n[[repo]]\nname = \"acme/webapp\"\nbranch = \"develop\"\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn one_repository_on_two_workflows_loads() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\n[[repo]]\nname = \"acme/webapp\"\nmain_workflow = \"deploy.yml\"\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn a_deploy_without_environments_is_refused() {
    assert!(
        refused("[[repo]]\nname = \"acme/webapp\"\n[[repo.deploy]]\n")
            .contains("the [[repo.deploy]] of acme/webapp has no [[repo.deploy.env]]")
    );
}

#[test]
fn a_deploy_on_the_second_repository_alone_loads() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\n[[repo]]\nname = \"acme/infra\"\n[[repo.deploy]]\n[[repo.deploy.env]]\nname = \"prod\"\ndeployment = \"api\"\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn two_repositories_differing_only_in_case_on_one_branch_are_one() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[[repo]]\nname = \"acme/webapp\"\n[[repo]]\nname = \"Acme/WebApp\"\n";

    let err = load(&write(&dir, text)).unwrap_err().to_string();

    assert!(
        err.contains("two [[repo]] named \"Acme/WebApp\" follow the same branch and main_workflow"),
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
            "{first}[[repo]]\nname = \"acme/infra\"\nrefresh_secs = 4\n"
        ))
        .contains("[[repo]] refresh_secs of acme/infra is 4; use 5 or more")
    );
}

#[test]
fn a_my_prs_limit_no_tab_falls_back_to_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let text = "[prs]\nlimit = 0\n[[prs.tab]]\nname = \"mine\"\nquery = \"is:pr\"\nlimit = 20\n";

    let loaded = load(&write(&dir, text));

    assert!(loaded.is_ok(), "{loaded:?}");
}

#[test]
fn a_my_prs_limit_a_tab_falls_back_to_is_held_to_the_range() {
    let text = "[prs]\nlimit = 0\n[[prs.tab]]\nname = \"mine\"\nquery = \"is:pr\"\nlimit = 20\n[[prs.tab]]\nname = \"team\"\nquery = \"is:pr\"\n";

    assert!(refused(text).contains("[prs] limit is 0; use 1 to 100"));
}

#[test]
fn a_quoted_name_is_written_escaped_on_one_line() {
    let repo = refused_repo("acme/\\nwebapp");
    let env = refused_env("name = \"pr\\nod\"\n");

    assert!(
        repo.contains("[[repo]] name \"acme/\\nwebapp\" is not owner/name"),
        "{repo}"
    );
    assert!(!repo.contains('\n'), "{repo}");
    assert!(
        env.contains("[[repo.deploy.env]] \"pr\\nod\" of acme/webapp has a blank deployment"),
        "{env}"
    );
    assert!(!env.contains('\n'), "{env}");
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
