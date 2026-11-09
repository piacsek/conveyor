use conveyor::model::deployed::parse_deployment;

const DEPLOYMENT: &str = include_str!("fixtures/deployment.json");

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
