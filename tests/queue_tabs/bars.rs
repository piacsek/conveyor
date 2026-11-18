use conveyor::config::Config;

use crate::helpers::{repo, two_repos};
use crate::support::{Harness, builds_in, queue_in};

#[test]
fn main_builds_draws_the_same_repository_bar_as_the_queue() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![
        queue_in(0, "acme/monorepo", vec![]),
        builds_in(0, "acme/monorepo", vec![]),
    ])
    .unwrap();

    let bar = h.screen().lines().nth(1).unwrap().to_string();
    assert_eq!(bar.matches("monorepo │ infra").count(), 2, "{bar}");
}

#[test]
fn one_repository_draws_no_bar_in_main_builds() {
    let config = Config {
        repo: vec![repo("acme/monorepo")],
        ..Config::default()
    };
    let mut h = Harness::with_config(160, 12, config);

    h.run(vec![builds_in(0, "acme/monorepo", vec![])]).unwrap();

    assert!(!h.screen().contains("monorepo │"), "{}", h.screen());
}
