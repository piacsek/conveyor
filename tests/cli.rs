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
