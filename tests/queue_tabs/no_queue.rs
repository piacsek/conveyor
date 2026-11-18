use ratatui::style::Modifier;

use crate::helpers::two_repos;
use crate::support::{Harness, no_queue_in};

#[test]
fn a_repository_without_a_merge_queue_reads_no_merge_queue_in_dim() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    let screen = h.screen();
    let body = screen.lines().nth(2).unwrap();
    assert!(
        body[body.find("││").unwrap()..].starts_with("││no merge queue"),
        "{screen}"
    );
    assert!(h.cell(41, 2).modifier.contains(Modifier::DIM));
}

#[test]
fn a_repository_without_a_merge_queue_is_no_error_and_has_no_count() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Queue monorepo─"), "{screen}");
    assert!(!screen.contains("⚠"), "{screen}");
}
