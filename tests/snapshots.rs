mod support;

use conveyor::app::Stage;
use conveyor::model::prs::{
    Check, CheckConclusion, CheckState, MergeState, PullRequest, ReviewDecision,
};
use ratatui::crossterm::event::KeyCode;
use support::{Harness, entry, failed, key, pr, prs, queue};

fn sample() -> Vec<PullRequest> {
    let mut failing = pr(4821, "Retry webhook delivery with backoff");
    failing.checks = CheckState::Failure;
    failing.head_ref = "webhook-retry-backoff".to_string();
    failing.additions = 12;
    failing.deletions = 3;
    failing.review = ReviewDecision::Approved;
    failing.merge_state = MergeState::Blocked;
    failing.checks_detail = vec![
        Check {
            name: "lint".to_string(),
            conclusion: CheckConclusion::Success,
            url: "https://github.com/acme/webapp/actions/runs/1/job/2".to_string(),
        },
        Check {
            name: "api / test".to_string(),
            conclusion: CheckConclusion::Failure,
            url: "https://github.com/acme/webapp/actions/runs/1/job/3".to_string(),
        },
    ];
    let mut pending = pr(4830, "Add rate limit headers to the API");
    pending.checks = CheckState::Pending;
    pending.repo = "acme/auth".to_string();
    let mut draft = pr(4790, "Spike: parallel test runner");
    draft.is_draft = true;
    draft.checks = CheckState::None;
    vec![failing, pending, draft]
}

fn queue_sample() -> Vec<conveyor::model::queue::QueueEntry> {
    let mut first = entry(1, 4821, "alice", "Retry webhook delivery with backoff");
    first.state = conveyor::model::queue::QueueState::AwaitingChecks;
    first.checks = CheckState::Pending;
    first.eta = Some(std::time::Duration::from_secs(12 * 60));
    first.run_url = Some("https://github.com/acme/webapp/actions/runs/1".to_string());
    let mut second = entry(2, 4830, "carol", "Add rate limit headers to the API");
    second.jump = true;
    vec![first, second]
}

#[test]
fn list_layout() {
    let mut h = Harness::new();
    h.run(vec![
        prs(sample()),
        queue(queue_sample()),
        key(KeyCode::Char('j')),
    ])
    .unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn queue_details_layout() {
    let mut h = Harness::with_size(160, 20);
    h.run(vec![
        prs(sample()),
        queue(queue_sample()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn filter_layout() {
    let mut h = Harness::new();
    h.run(vec![
        prs(sample()),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('r')),
    ])
    .unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn details_layout() {
    let mut h = Harness::with_size(160, 20);
    h.run(vec![prs(sample()), key(KeyCode::Char('p'))]).unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn error_layout() {
    let mut h = Harness::new();
    h.run(vec![
        prs(sample()),
        failed(Stage::Prs, "gh: HTTP 401: Bad credentials"),
    ])
    .unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn empty_layout() {
    let mut h = Harness::new();
    h.run(vec![prs(vec![])]).unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn loading_layout() {
    let mut h = Harness::new();
    h.run(vec![]).unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn tabs_layout() {
    let mut h = Harness::with_size(80, 12);
    h.run(vec![prs(sample())]).unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn help_layout() {
    let mut h = Harness::with_size(160, 16);
    h.run(vec![prs(sample()), key(KeyCode::Char('?'))]).unwrap();
    insta::assert_snapshot!(h.screen());
}

#[test]
fn zoom_layout() {
    let mut h = Harness::new();
    h.run(vec![prs(sample()), key(KeyCode::Char('z'))]).unwrap();
    insta::assert_snapshot!(h.screen());
}
