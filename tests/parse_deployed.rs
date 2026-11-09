use conveyor::model::deployed::{Condition, Health, Pod, health, parse_deployment, parse_pods};
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

#[test]
fn a_crashlooping_pod_under_an_available_deployment_is_a_warning() {
    let rollout = parse_deployment(DEPLOYMENT).unwrap();
    let pods = parse_pods(PODS).unwrap();

    assert_eq!(
        health(&rollout, Some(&pods)),
        Health::Warning("1 of 3 pods CrashLoopBackOff".to_string())
    );
}

#[test]
fn a_deployment_that_is_not_available_is_an_alert_whatever_its_pods_say() {
    let mut rollout = parse_deployment(DEPLOYMENT).unwrap();
    rollout.available = 0;
    rollout.conditions[0].status = "False".to_string();
    let pods = parse_pods(PODS).unwrap();

    assert_eq!(
        health(&rollout, Some(&pods)),
        Health::Alert("0 of 2 available".to_string())
    );
    assert_eq!(
        health(&rollout, None),
        Health::Alert("0 of 2 available".to_string())
    );
}

#[test]
fn an_available_deployment_whose_pods_were_not_read_is_unread() {
    let mut rollout = parse_deployment(DEPLOYMENT).unwrap();
    rollout.conditions[1].status = "True".to_string();

    assert_eq!(health(&rollout, None), Health::Unread);
}

#[test]
fn a_rollout_past_its_progress_deadline_is_a_warning_even_without_a_stuck_pod() {
    let rollout = parse_deployment(DEPLOYMENT).unwrap();
    let pods: Vec<Pod> = parse_pods(PODS)
        .unwrap()
        .into_iter()
        .filter(|pod| pod.waiting.is_none())
        .collect();

    assert_eq!(
        health(&rollout, Some(&pods)),
        Health::Warning("rollout stalled".to_string())
    );
    assert_eq!(
        health(&rollout, None),
        Health::Warning("rollout stalled".to_string())
    );
}

#[test]
fn a_terminating_pod_never_counts_towards_health() {
    let mut rollout = parse_deployment(DEPLOYMENT).unwrap();
    rollout.conditions[1].status = "True".to_string();
    let mut pods = parse_pods(PODS).unwrap();
    pods[0].terminating = true;

    assert_eq!(health(&rollout, Some(&pods)), Health::Healthy);
}

#[test]
fn a_deployment_scaled_to_zero_is_healthy() {
    let mut rollout = parse_deployment(DEPLOYMENT).unwrap();
    rollout.desired = 0;
    rollout.available = 0;
    rollout.conditions[1].status = "True".to_string();

    assert_eq!(health(&rollout, Some(&[])), Health::Healthy);
}

#[test]
fn a_pod_caught_running_between_crashes_is_still_stuck() {
    let mut rollout = parse_deployment(DEPLOYMENT).unwrap();
    rollout.conditions[1].status = "True".to_string();
    let pods = parse_pods(
        &serde_json::json!({"items": [{
            "metadata": {"name": "api-1"},
            "status": {"phase": "Running", "containerStatuses": [{
                "ready": false,
                "restartCount": 3,
                "state": {"running": {}},
                "lastState": {"terminated": {"exitCode": 1, "reason": "Error"}}
            }]}
        }]})
        .to_string(),
    )
    .unwrap();

    assert_eq!(pods[0].stuck.as_deref(), Some("restarting"));
    assert_eq!(
        pods[0].last_exit.as_ref().map(|exit| exit.exit_code),
        Some(1)
    );
    assert_eq!(
        health(&rollout, Some(&pods)),
        Health::Warning("1 of 1 pods restarting".to_string())
    );
}

#[test]
fn a_pod_is_judged_by_its_stuck_container_not_its_first_waiting_one() {
    let pods = parse_pods(
        &serde_json::json!({"items": [{
            "metadata": {"name": "api-1"},
            "status": {"phase": "Pending", "containerStatuses": [
                {"ready": false, "restartCount": 0,
                 "state": {"waiting": {"reason": "ContainerCreating"}}},
                {"ready": false, "restartCount": 4,
                 "state": {"waiting": {"reason": "CrashLoopBackOff", "message": "back-off 40s"}},
                 "lastState": {"terminated": {"exitCode": 2, "reason": "Error"}}}
            ]}
        }]})
        .to_string(),
    )
    .unwrap();

    assert_eq!(pods[0].stuck.as_deref(), Some("CrashLoopBackOff"));
    assert_eq!(
        pods[0]
            .waiting
            .as_ref()
            .map(|waiting| waiting.reason.as_str()),
        Some("CrashLoopBackOff")
    );
    assert_eq!(
        pods[0].last_exit.as_ref().map(|exit| exit.exit_code),
        Some(2)
    );
}

#[test]
fn pods_stuck_for_different_reasons_are_counted_without_naming_one() {
    let mut rollout = parse_deployment(DEPLOYMENT).unwrap();
    rollout.conditions[1].status = "True".to_string();
    let pods = [
        Pod {
            stuck: Some("CrashLoopBackOff".to_string()),
            ..Pod::default()
        },
        Pod {
            stuck: Some("ImagePullBackOff".to_string()),
            ..Pod::default()
        },
        Pod {
            phase: "Failed".to_string(),
            ..Pod::default()
        },
        Pod {
            phase: "Succeeded".to_string(),
            ..Pod::default()
        },
    ];

    assert_eq!(
        health(&rollout, Some(&pods)),
        Health::Warning("2 of 2 pods stuck".to_string())
    );
}

#[test]
fn the_last_exit_comes_from_the_restarting_container_not_one_that_merely_waits() {
    let pods = parse_pods(
        &serde_json::json!({"items": [{
            "metadata": {"name": "api-1"},
            "status": {
                "phase": "Pending",
                "initContainerStatuses": [{
                    "ready": false, "restartCount": 3, "state": {"running": {}},
                    "lastState": {"terminated": {"exitCode": 1, "reason": "Error"}}
                }],
                "containerStatuses": [{
                    "ready": false, "restartCount": 0,
                    "state": {"waiting": {"reason": "PodInitializing"}}
                }]
            }
        }]})
        .to_string(),
    )
    .unwrap();

    assert_eq!(pods[0].stuck.as_deref(), Some("restarting"));
    assert_eq!(
        pods[0].last_exit.as_ref().map(|exit| exit.exit_code),
        Some(1)
    );
    assert_eq!(pods[0].waiting, None);
}

#[test]
fn a_replica_failure_is_a_warning_while_the_deployment_is_still_available() {
    let mut rollout = parse_deployment(DEPLOYMENT).unwrap();
    rollout.conditions[1].status = "True".to_string();
    rollout.conditions.push(Condition {
        kind: "ReplicaFailure".to_string(),
        status: "True".to_string(),
        reason: "FailedCreate".to_string(),
        message: "exceeded quota".to_string(),
    });

    assert_eq!(
        health(&rollout, Some(&[])),
        Health::Warning("replicas cannot be created".to_string())
    );
}
