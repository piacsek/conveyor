use std::sync::mpsc;
use std::time::Duration;

use conveyor::app::{App, Input, Rows, Stage};
use conveyor::config::Config;
use conveyor::refresh;
use conveyor::sources::failure::{Failure, FailureKind};
use conveyor::sources::fetch::fetch_prs;
use conveyor::sources::github::CliGh;

use crate::shims::{serialized, shim};

const SIGNED_OUT: &str = "echo 'To get started with GitHub CLI, please run:  gh auth login' >&2; \
     echo 'Alternatively, populate the GH_TOKEN environment variable with a GitHub API authentication token.' >&2; \
     exit 4";

#[test]
fn a_signed_out_gh_fails_my_prs_as_not_signed_in_with_its_own_words() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Config::default());

    app.receive(Stage::Prs, 0, fetched_prs(shim(&dir, SIGNED_OUT)));

    let failure = app.prs.column().error.clone().unwrap();
    assert_eq!(failure.kind(), FailureKind::NotSignedIn);
    assert_eq!(
        app.focused_error(),
        Some(
            "To get started with GitHub CLI, please run:  gh auth login\n\
             Alternatively, populate the GH_TOKEN environment variable with a GitHub API authentication token."
        )
    );
}

fn fetched_prs(gh: CliGh) -> Result<Rows, Failure> {
    let (tx, rx) = mpsc::channel();
    let config = Config::default();
    let _refresh = refresh::spawn(Stage::Prs, Duration::from_secs(60), 1, tx, move |tab| {
        fetch_prs(&gh, &config, tab).map(Rows::Prs)
    });
    loop {
        if let Input::Data(_, _, result) = rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap()
        {
            return result;
        }
    }
}
