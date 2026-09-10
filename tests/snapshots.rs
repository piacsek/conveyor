mod support;

use conveyor::app::Stage;
use conveyor::model::prs::{
    Check, CheckConclusion, CheckState, MergeState, PullRequest, ReviewDecision,
};
use ratatui::crossterm::event::KeyCode;
use support::{Harness, failed, key, pr, prs};

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

#[test]
fn list_layout() {
    let mut h = Harness::new();
    h.run(vec![prs(sample()), key(KeyCode::Char('j'))]).unwrap();
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
