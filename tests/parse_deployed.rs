use conveyor::model::deployed::{parse_deployment, parse_pods};
use conveyor::model::prs::parse_timestamp;

const DEPLOYMENT: &str = include_str!("fixtures/deployment.json");
const PODS: &str = include_str!("fixtures/pods.json");

#[test]
fn a_deployment_gives_its_image_selector_replica_counts_and_conditions() {
    let rollout = parse_deployment(DEPLOYMENT).unwrap();

    assert_eq!(
        rollout.image,
        "ghcr.io/acme/api:8c1f2a7d4e9b01c35f6a7b8c9d0e1f2a3b4c5d6e"
    );
    assert_eq!(
        rollout.selector,
        "app.kubernetes.io/instance=api,app.kubernetes.io/name=api"
    );
    assert_eq!(rollout.desired, 2);
    assert_eq!(rollout.updated, 1);
    assert_eq!(rollout.ready, 2);
    assert_eq!(rollout.available, 2);
    assert_eq!(rollout.conditions.len(), 2);
    assert_eq!(rollout.conditions[0].kind, "Available");
    assert_eq!(rollout.conditions[0].status, "True");
    assert_eq!(rollout.conditions[1].kind, "Progressing");
    assert_eq!(rollout.conditions[1].status, "False");
    assert_eq!(rollout.conditions[1].reason, "ProgressDeadlineExceeded");
    assert_eq!(
        rollout.conditions[1].message,
        "ReplicaSet \"api-7d9c6b5f48\" has timed out progressing."
    );
}

#[test]
fn pods_parse_problems_first_with_the_waiting_reason_and_the_last_exit() {
    let pods = parse_pods(PODS).unwrap();

    assert_eq!(pods.len(), 3);
    let crashing = &pods[0];
    assert_eq!(crashing.name, "api-7d9c6b5f48-x2k9p");
    assert_eq!(crashing.phase, "Running");
    assert!(!crashing.ready);
    assert_eq!(crashing.restarts, 7);
    assert_eq!(crashing.started_at, parse_timestamp("2026-11-09T10:21:40Z"));
    assert_eq!(
        crashing.sha.as_deref(),
        Some("8c1f2a7d4e9b01c35f6a7b8c9d0e1f2a3b4c5d6e")
    );
    let waiting = crashing.waiting.as_ref().unwrap();
    assert_eq!(waiting.reason, "CrashLoopBackOff");
    assert!(
        waiting.message.starts_with("back-off 5m0s"),
        "{}",
        waiting.message
    );
    let last_exit = crashing.last_exit.as_ref().unwrap();
    assert_eq!(last_exit.reason, "OOMKilled");
    assert_eq!(last_exit.exit_code, 137);
    assert!(!crashing.terminating);
    assert_eq!(pods[1].name, "api-5b8f7c6d94-hq4zt");
    assert!(pods[1].ready);
    assert_eq!(pods[1].restarts, 0);
    assert_eq!(pods[1].waiting, None);
    assert_eq!(pods[1].last_exit, None);
    assert_eq!(
        pods[1].sha.as_deref(),
        Some("4e9b01c3a7d2f5e68b9c0d1e2f3a4b5c6d7e8f90")
    );
    assert_eq!(pods[2].name, "api-5b8f7c6d94-m7wvn");
}
