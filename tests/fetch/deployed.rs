use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::fakes::{FakeGithub, FakeKube, NOW, SHA, deploy_source_for, deployment_json};

#[test]
fn deploy_source_reads_its_environments_at_the_same_time() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    let kube = OverlappingKube::default();
    let deploy = Deploy {
        env: ["dev", "staging", "prod"]
            .iter()
            .map(|name| DeployEnv {
                name: name.to_string(),
                ..DeployEnv::default()
            })
            .collect(),
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);

    assert_eq!(kube.most_at_once.load(Ordering::SeqCst), 3);
}

#[test]
fn deploy_source_reads_each_environment_and_keeps_per_env_errors_on_the_row() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = vec![
        (
            "repos/acme/webapp/commits/0123456789abcdef0123456789abcdef01234567".to_string(),
            serde_json::from_str(include_str!("../fixtures/commit-pulls.json")).unwrap(),
        ),
        (
            "repos/acme/webapp/commits/".to_string(),
            serde_json::json!([]),
        ),
    ];
    let kube = FakeKube::with(vec![
        (
            "staging".to_string(),
            Ok(deployment_json(
                "ghcr.io/acme/api:0123456789abcdef0123456789abcdef01234567",
            )),
        ),
        (
            "prod".to_string(),
            Err(io::Error::other("ERROR: Active profile expired.")),
        ),
        (
            "dev".to_string(),
            Ok(deployment_json("ghcr.io/acme/api:latest")),
        ),
    ]);
    let deploy = Deploy {
        system: "api".to_string(),
        env: ["staging", "prod", "dev"]
            .iter()
            .map(|name| DeployEnv {
                name: name.to_string(),
                ..DeployEnv::default()
            })
            .collect(),
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    let deployed = source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);

    assert_eq!(deployed.system, "api");
    assert_eq!(deployed.rows.len(), 3);
    assert_eq!(deployed.rows[0].env, "staging");
    assert_eq!(
        deployed.rows[0].sha.as_deref(),
        Some("0123456789abcdef0123456789abcdef01234567")
    );
    assert_eq!(deployed.rows[0].pull.as_ref().map(|p| p.number), Some(3));
    assert_eq!(deployed.rows[0].error, None);
    assert_eq!(
        deployed.rows[1].error.as_deref(),
        Some("ERROR: Active profile expired.")
    );
    assert_eq!(deployed.rows[1].sha, None);
    assert!(
        deployed.rows[2]
            .error
            .as_deref()
            .unwrap()
            .contains("unexpected image tag")
    );
    assert_eq!(
        deployed.rows[2].image.as_deref(),
        Some("ghcr.io/acme/api:latest")
    );

    source.fetch(&gh, &kube, std::time::SystemTime::UNIX_EPOCH);
    let calls = gh.calls.borrow().clone();
    assert_eq!(
        calls
            .iter()
            .filter(|(q, _)| q.contains("/commits/"))
            .count(),
        1,
        "sha lookup cached"
    );
}

#[test]
fn deploy_source_reads_the_pods_behind_the_deployment_and_judges_its_health() {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::model::deployed::Health;
    use conveyor::sources::fetch::DeploySource;
    let gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    let mut kube = FakeKube::with(vec![(
        "prod".to_string(),
        Ok(include_str!("../fixtures/deployment.json").to_string()),
    )]);
    kube.pods = Ok(include_str!("../fixtures/pods.json").to_string());
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            context: "ctx-prod".to_string(),
            namespace: "web".to_string(),
            deployment: "api".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    let mut source = DeploySource::new("acme/webapp".to_string(), deploy);

    let deployed = source.fetch(&gh, &kube, NOW);

    let row = &deployed.rows[0];
    assert_eq!(
        row.sha.as_deref(),
        Some("8c1f2a7d4e9b01c35f6a7b8c9d0e1f2a3b4c5d6e")
    );
    assert_eq!(row.error, None);
    assert_eq!(
        row.health,
        Health::Warning("1 of 3 pods CrashLoopBackOff".to_string())
    );
    assert_eq!(row.pods.len(), 3);
    assert_eq!(row.pods_error, None);
    assert_eq!(row.rollout.as_ref().map(|rollout| rollout.desired), Some(2));
    assert_eq!(row.target, "ctx-prod / web / api");
    assert_eq!(
        *kube.selectors.lock().unwrap(),
        vec!["app.kubernetes.io/instance=api,app.kubernetes.io/name=api"]
    );
}

#[test]
fn pods_that_cannot_be_read_leave_the_row_unread_instead_of_failing_it() {
    use conveyor::model::deployed::Health;
    let (gh, mut kube, mut source) = deploy_source_for(SHA, Vec::new());
    kube.pods = Err("Error from server (Forbidden): pods is forbidden".to_string());

    let deployed = source.fetch(&gh, &kube, NOW);

    let row = &deployed.rows[0];
    assert_eq!(row.error, None);
    assert_eq!(row.sha.as_deref(), Some(SHA));
    assert_eq!(row.health, Health::Unread);
    assert_eq!(
        row.pods_error.as_deref(),
        Some("Error from server (Forbidden): pods is forbidden")
    );
}

#[test]
fn a_deployment_without_match_labels_is_unread_and_asks_for_no_pods() {
    use conveyor::model::deployed::Health;
    let (gh, mut kube, mut source) = deploy_source_for(SHA, Vec::new());
    kube.deployments = vec![(
        "prod".to_string(),
        Ok(serde_json::json!({
            "spec": {"template": {"spec": {"containers": [{"image": format!("ghcr.io/acme/api:{SHA}")}]}}},
            "status": {"conditions": [{"type": "Available", "status": "True"}]}
        })
        .to_string()),
    )];

    let deployed = source.fetch(&gh, &kube, NOW);

    let row = &deployed.rows[0];
    assert_eq!(row.health, Health::Unread);
    assert_eq!(
        row.pods_error.as_deref(),
        Some("the deployment's selector has no matchLabels")
    );
    assert_eq!(kube.selectors.lock().unwrap().len(), 0);
}

#[derive(Default)]
struct OverlappingKube {
    running: std::sync::Mutex<usize>,
    another_started: std::sync::Condvar,
    most_at_once: AtomicUsize,
}

impl conveyor::sources::kube::Kube for OverlappingKube {
    fn deployment(&self, _env: &conveyor::config::DeployEnv) -> io::Result<String> {
        let mut running = self.running.lock().unwrap();
        *running += 1;
        self.most_at_once.fetch_max(*running, Ordering::SeqCst);
        self.another_started.notify_all();
        let (mut running, _) = self
            .another_started
            .wait_timeout_while(running, std::time::Duration::from_secs(2), |_| {
                self.most_at_once.load(Ordering::SeqCst) < 3
            })
            .unwrap();
        *running -= 1;
        Err(io::Error::other("unreachable"))
    }

    fn pods(&self, _env: &conveyor::config::DeployEnv, _selector: &str) -> io::Result<String> {
        Ok(r#"{"items":[]}"#.to_string())
    }
}
