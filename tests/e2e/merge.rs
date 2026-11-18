use std::fs;
use std::path::Path;

use super::{Server, fixture_path, shim};

#[test]
#[ignore]
fn s_m_then_y_runs_gh_pr_merge_pinned_to_the_head_commit() {
    let home = tempfile::tempdir().unwrap();

    let argv = merge(home.path(), "merge");

    assert_eq!(
        argv,
        "pr merge 1 -R piacsek/conveyor --squash --match-head-commit 54b33470c91a2ce5a25ddd4bfea8acf6c3b9d481\n"
    );
}

#[test]
#[ignore]
fn s_m_then_y_merges_with_the_method_of_the_pull_requests_repository() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".config/conveyor");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("config.toml"),
        "[[repo]]\nname = \"piacsek/conveyor\"\nmerge_method = \"rebase\"\n",
    )
    .unwrap();

    let argv = merge(home.path(), "merge-rebase");

    assert_eq!(
        argv,
        "pr merge 1 -R piacsek/conveyor --rebase --match-head-commit 54b33470c91a2ce5a25ddd4bfea8acf6c3b9d481\n"
    );
}

fn merge(home: &Path, session: &str) -> String {
    let fixture = fixture_path(home);
    let argv = home.join("merge-argv");
    let path = shim(
        home,
        &format!(
            "case \"$*\" in 'pr merge'*) echo \"$*\" > '{}';; *) cat '{fixture}';; esac",
            argv.display()
        ),
    );
    let server = Server::start(session);
    server.respawn(
        &[("HOME", &home.display().to_string()), ("PATH", &path)],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["S", "m"]);
    server.wait_for_screen("merge #1 Phase 0: setup?");
    server.send_keys(&["y"]);
    server.wait_for_screen("merge sent for #1");

    fs::read_to_string(&argv).unwrap()
}
