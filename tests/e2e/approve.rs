use std::fs;

use super::{Server, fixture_path, shim};

#[test]
#[ignore]
fn s_v_then_y_approves_the_head_commit_the_card_showed() {
    let home = tempfile::tempdir().unwrap();
    let fixture = fixture_path(home.path());
    let argv = home.path().join("approve-argv");
    let path = shim(
        home.path(),
        &format!(
            "case \"$*\" in 'pr review'*|*addPullRequestReview*) echo \"$*\" > '{}';; *) cat '{fixture}';; esac",
            argv.display()
        ),
    );
    let server = Server::start("approve");
    server.respawn(
        &[
            ("HOME", &home.path().display().to_string()),
            ("PATH", &path),
        ],
        env!("CARGO_BIN_EXE_conveyor"),
    );
    server.wait_for_screen("#1 Phase 0: setup");

    server.send_keys(&["S", "v"]);
    server.wait_for_screen("approve #1 Phase 0: setup?");
    server.send_keys(&["y"]);
    server.wait_for_screen("#1 approved");

    let sent = fs::read_to_string(&argv).unwrap();
    assert!(
        sent.contains(
            "addPullRequestReview(input: {pullRequestId: $id, event: APPROVE, commitOID: $head})"
        ),
        "{sent}"
    );
    assert!(
        sent.ends_with(" -f head=54b33470c91a2ce5a25ddd4bfea8acf6c3b9d481\n"),
        "{sent}"
    );
}
