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
        CONFIG_V0.replace(
            "details_percent = 30\n",
            &format!("details_percent = 30\n{KEYS}")
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

[ui.keys]
next_tab = \"tab\"
previous_tab = \"shift-tab\"
";

const KEYS: &str = "
[ui.keys]
next_tab = \"tab\"
previous_tab = \"shift-tab\"
";

fn write_config(dir: &tempfile::TempDir, text: &str) -> std::path::PathBuf {
    let path = dir.path().join("config.toml");
    std::fs::write(&path, text).unwrap();
    path
}
