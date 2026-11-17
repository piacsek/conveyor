use std::process::Command;

fn binary(args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .args(args)
        .env("HOME", dir.path())
        .env_remove("CONVEYOR_CONFIG")
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .unwrap()
}

#[test]
fn help_flags_print_usage_on_stdout_and_exit_zero() {
    for flag in ["--help", "-h", "help"] {
        let out = binary(&[flag]);
        assert!(out.status.success(), "{flag}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("usage: conveyor"), "{flag}: {stdout}");
        assert!(stdout.contains("config"), "{flag}: {stdout}");
    }
}

#[test]
fn version_flags_print_the_crate_version() {
    for flag in ["--version", "-V", "version"] {
        let out = binary(&[flag]);
        assert!(out.status.success(), "{flag}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            format!("conveyor {}", env!("CARGO_PKG_VERSION")),
            "{flag}"
        );
    }
}

#[test]
fn an_unknown_argument_prints_usage_on_stderr_and_exits_one() {
    let out = binary(&["bogus"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown argument 'bogus'"), "{stderr}");
    assert!(stderr.contains("usage: conveyor"), "{stderr}");
    assert!(out.stdout.is_empty());
}

#[test]
fn an_argument_after_the_first_prints_usage_on_stderr_and_exits_one() {
    let out = binary(&["--demo", "--help"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown argument '--help'"), "{stderr}");
    assert!(stderr.contains("usage: conveyor"), "{stderr}");
    assert!(out.stdout.is_empty());
}

#[test]
fn config_prints_the_effective_defaults_as_toml() {
    let out = binary(&["config"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[prs]"), "{stdout}");
    assert!(
        stdout.contains("query = \"is:pr is:open archived:false (author:@me OR assignee:@me)\""),
        "{stdout}"
    );
    assert!(stdout.contains("refresh_secs = 60"), "{stdout}");
    assert!(stdout.contains("merge_method = \"squash\""), "{stdout}");
    assert!(stdout.contains("update_method = \"merge\""), "{stdout}");
    assert!(stdout.contains("background_tabs = true"), "{stdout}");
}

#[test]
fn config_prints_plain_icons_by_default() {
    let out = binary(&["config"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("details_percent = 40\nicons = \"plain\"\n"),
        "{stdout}"
    );
}

#[test]
fn config_refuses_unknown_icons_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "[ui]\nicons = \"emoji\"\n");

    let out = config_command(&path);

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("config.toml"), "{stderr}");
    assert!(stderr.contains("emoji"), "{stderr}");
    assert!(out.stdout.is_empty());
}

#[test]
fn config_prints_background_tabs_off_when_set_off() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "[prs]\nbackground_tabs = false\n");

    let out = config_command(&path);

    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("background_tabs = false"), "{stdout}");
}

#[test]
fn config_refuses_a_misspelled_background_tabs() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "[prs]\nbackground_tab = false\n");

    let out = config_command(&path);

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("background_tab"), "{stderr}");
}

#[test]
fn config_reads_the_file_named_by_conveyor_config_and_reports_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mine.toml");
    std::fs::write(&path, "[prs]\nlimit = 3\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .arg("config")
        .env("HOME", dir.path())
        .env("CONVEYOR_CONFIG", &path)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("limit = 3"));

    std::fs::write(&path, "[prs]\nlimt = 3\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .arg("config")
        .env("HOME", dir.path())
        .env("CONVEYOR_CONFIG", &path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("mine.toml"), "{stderr}");
    assert!(stderr.contains("limt"), "{stderr}");
    assert!(out.stdout.is_empty());
}

#[test]
fn a_config_written_before_tabs_loads_and_prints_back_unchanged() {
    let out = config_command(&fixture("config-v0.toml"));

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        CONFIG_V0
            .replace(
                "update_method = \"rebase\"\n",
                "update_method = \"rebase\"\nbackground_tabs = true\n"
            )
            .replace(
                "details_percent = 30\n",
                &format!("details_percent = 30\n{ICONS_AND_KEYS}")
            )
    );
}

#[test]
fn config_prints_the_my_prs_tabs() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, TWO_TABS);

    let out = config_command(&path);

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), PRINTED_TWO_TABS);
}

#[test]
fn config_without_tabs_prints_no_tab_list() {
    let out = binary(&["config"]);

    assert!(!String::from_utf8_lossy(&out.stdout).contains("prs.tab"));
}

#[test]
fn a_query_beside_tabs_is_an_error_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, &format!("[prs]\nquery = \"is:pr\"\n\n{TWO_TABS}"));

    let out = config_command(&path);

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("config.toml"), "{stderr}");
    assert!(stderr.contains("[prs].query"), "{stderr}");
}

#[test]
fn a_tab_without_a_query_is_an_error_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "[[prs.tab]]\nname = \"mine\"\n");

    let out = config_command(&path);

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("config.toml"), "{stderr}");
    assert!(stderr.contains("missing field `query`"), "{stderr}");
}

#[test]
fn a_tab_without_a_name_is_an_error_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "[[prs.tab]]\nquery = \"is:pr\"\n");

    let out = config_command(&path);

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("config.toml"), "{stderr}");
    assert!(stderr.contains("missing field `name`"), "{stderr}");
}

#[test]
fn two_tabs_with_the_same_name_are_an_error_naming_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let repeated = TWO_TABS.replace("\"review\"", "\"mine\"");
    let path = write_config(&dir, &repeated);

    let out = config_command(&path);

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("config.toml"), "{stderr}");
    assert!(stderr.contains("\"mine\""), "{stderr}");
}

#[test]
fn log_prints_the_log_path_named_after_the_program() {
    let dir = tempfile::tempdir().unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .arg("log")
        .env("HOME", dir.path())
        .env_remove("CONVEYOR_LOG")
        .env_remove("XDG_STATE_HOME")
        .output()
        .unwrap();

    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        dir.path()
            .join(".local/state/conveyor/conveyor.log")
            .display()
            .to_string()
    );
}

#[test]
fn log_without_an_absolute_home_says_so_and_exits_one() {
    let out = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .arg("log")
        .env("HOME", "")
        .env_remove("CONVEYOR_LOG")
        .env_remove("XDG_STATE_HOME")
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no log: HOME is not an absolute path"),
        "{stderr}"
    );
}

#[test]
fn config_prints_the_branch_of_a_repository_that_sets_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(
        &dir,
        "[[repo]]\nname = \"acme/webapp\"\nbranch = \"develop\"\n",
    );

    let out = config_command(&path);

    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("branch = \"develop\"\n"), "{stdout}");
}

#[test]
fn config_leaves_the_branch_out_while_it_is_unset() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "[[repo]]\nname = \"acme/webapp\"\n");

    let out = config_command(&path);

    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("branch"), "{stdout}");
}

#[test]
fn config_refuses_my_prs_refreshing_more_often_than_every_five_seconds() {
    let (code, stderr) = refusal("[prs]\nrefresh_secs = 4\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("config.toml: [prs] refresh_secs is 4; use 5 or more"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_repository_refreshing_more_often_than_every_five_seconds() {
    let (code, stderr) = refusal("[[repo]]\nname = \"acme/webapp\"\nrefresh_secs = 4\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[[repo]] refresh_secs of acme/webapp is 4; use 5 or more"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_deploy_refreshing_more_often_than_every_five_seconds() {
    let (code, stderr) =
        refusal("[[repo]]\nname = \"acme/webapp\"\n[[repo.deploy]]\nrefresh_secs = 4\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[[repo.deploy]] refresh_secs of acme/webapp is 4; use 5 or more"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_my_prs_limit_github_would_not_answer() {
    let (code, stderr) = refusal("[prs]\nlimit = 101\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[prs] limit is 101; use 1 to 100"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_tab_limit_github_would_not_answer() {
    let (code, stderr) = refusal("[[prs.tab]]\nname = \"mine\"\nquery = \"is:pr\"\nlimit = 0\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[[prs.tab]] limit of \"mine\" is 0; use 1 to 100"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_number_of_builds_github_would_not_answer() {
    let (code, stderr) = refusal("[[repo]]\nname = \"acme/webapp\"\nbuilds = 0\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[[repo]] builds of acme/webapp is 0; use 1 to 100"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_details_pane_that_leaves_no_room_for_the_columns() {
    let (code, stderr) = refusal("[ui]\ndetails_percent = 500\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[ui] details_percent is 500; use 1 to 99"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_column_width_too_wide_to_count() {
    let (code, stderr) = refusal("[ui]\nmin_column_width = 16384\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[ui] min_column_width is 16384; use 16383 or less"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_repository_name_without_its_owner() {
    let (code, stderr) = refusal("[[repo]]\nname = \"not-a-repo\"\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[[repo]] name \"not-a-repo\" is not owner/name"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_an_environment_with_a_blank_deployment() {
    let (code, stderr) = refusal(&format!(
        "{ONE_DEPLOY}[[repo.deploy.env]]\nname = \"prod\"\nnamespace = \"api\"\n"
    ));

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("[[repo.deploy.env]] \"prod\" of acme/webapp has a blank deployment"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_two_repositories_nothing_tells_apart() {
    let (code, stderr) =
        refusal("[[repo]]\nname = \"acme/webapp\"\n[[repo]]\nname = \"acme/webapp\"\n");

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains(
            "two [[repo]] named \"acme/webapp\" follow the same branch and main_workflow"
        ),
        "{stderr}"
    );
}

#[test]
fn config_refuses_two_environments_of_one_name() {
    let env = "[[repo.deploy.env]]\nname = \"prod\"\ncontext = \"ctx\"\nnamespace = \"api\"\ndeployment = \"api\"\n";
    let (code, stderr) = refusal(&format!("{ONE_DEPLOY}{env}{env}"));

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("two [[repo.deploy.env]] of acme/webapp are named \"prod\""),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_second_deploy_it_would_ignore() {
    let (code, stderr) = refusal(&format!("{ONE_DEPLOY}[[repo.deploy]]\n"));

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("acme/webapp has a second [[repo.deploy]]; conveyor reads only the first"),
        "{stderr}"
    );
}

#[test]
fn config_refuses_a_deploy_on_two_repositories() {
    let (code, stderr) = refusal(&format!(
        "{ONE_DEPLOY}{PROD}[[repo]]\nname = \"acme/infra\"\n[[repo.deploy]]\n{PROD}"
    ));

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains(
            "the [[repo.deploy]] of acme/infra is ignored: conveyor reads only the one of acme/webapp"
        ),
        "{stderr}"
    );
}

#[test]
fn config_refusing_a_deploy_on_one_repository_listed_twice_names_the_entry() {
    let (code, stderr) = refusal(&format!(
        "{ONE_DEPLOY}{PROD}[[repo]]\nname = \"acme/webapp\"\nbranch = \"develop\"\n[[repo.deploy]]\n{PROD}"
    ));

    assert_eq!(code, Some(1));
    assert!(
        stderr.contains(
            "the [[repo.deploy]] of acme/webapp ([[repo]] number 2) is ignored: conveyor reads only the one of acme/webapp ([[repo]] number 1)"
        ),
        "{stderr}"
    );
}

const ONE_DEPLOY: &str = "[[repo]]\nname = \"acme/webapp\"\n[[repo.deploy]]\n";

const PROD: &str = "[[repo.deploy.env]]\nname = \"prod\"\ndeployment = \"api\"\n";

const CONFIG_V0: &str = include_str!("fixtures/config-v0.toml");

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn config_command(path: &std::path::Path) -> std::process::Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .arg("config")
        .env("HOME", home.path())
        .env("CONVEYOR_CONFIG", path)
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .unwrap()
}

const TWO_TABS: &str = "[[prs.tab]]
name = \"mine\"
query = \"is:pr is:open author:@me\"

[[prs.tab]]
name = \"review\"
query = \"is:pr is:open review-requested:@me\"
limit = 10
";

const PRINTED_TWO_TABS: &str = "repo = []

[prs]
limit = 20
refresh_secs = 60
merge_method = \"squash\"
update_method = \"merge\"
background_tabs = true

[[prs.tab]]
name = \"mine\"
query = \"is:pr is:open author:@me\"

[[prs.tab]]
name = \"review\"
query = \"is:pr is:open review-requested:@me\"
limit = 10

[ui]
min_column_width = 36
details_percent = 40
icons = \"plain\"

[ui.keys]
next_tab = \"tab\"
previous_tab = \"shift-tab\"
";

const ICONS_AND_KEYS: &str = "icons = \"plain\"

[ui.keys]
next_tab = \"tab\"
previous_tab = \"shift-tab\"
";

fn write_config(dir: &tempfile::TempDir, text: &str) -> std::path::PathBuf {
    let path = dir.path().join("config.toml");
    std::fs::write(&path, text).unwrap();
    path
}

fn refusal(text: &str) -> (Option<i32>, String) {
    let dir = tempfile::tempdir().unwrap();
    let out = config_command(&write_config(&dir, text));
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}
