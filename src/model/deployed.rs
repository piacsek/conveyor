use std::time::SystemTime;

use crate::model::builds::PullRef;
use crate::model::prs::parse_timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deployment {
    pub env: String,
    pub image: Option<String>,
    pub sha: Option<String>,
    pub pull: Option<PullRef>,
    pub error: Option<String>,
    pub fetched_at: Option<SystemTime>,
    pub target: String,
    pub health: Health,
    pub rollout: Option<Rollout>,
    pub pods: Vec<Pod>,
    pub pods_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deployed {
    pub system: String,
    pub rows: Vec<Deployment>,
}

pub fn sha_from_image(image: &str) -> Result<String, String> {
    let (_, tag) = image
        .rsplit_once(':')
        .filter(|(repo, _)| !repo.contains('@'))
        .ok_or_else(|| format!("unexpected image tag in `{image}`: no tag"))?;
    let candidate = tag.rsplit('-').next().unwrap_or(tag);
    if candidate.len() == 40 && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(candidate.to_string())
    } else {
        Err(format!(
            "unexpected image tag `{tag}`: expected a 40-hex commit sha"
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Condition {
    pub kind: String,
    pub status: String,
    pub reason: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Rollout {
    pub image: String,
    pub selector: String,
    pub desired: u64,
    pub updated: u64,
    pub ready: u64,
    pub available: u64,
    pub conditions: Vec<Condition>,
}

impl Rollout {
    fn stalled(&self) -> bool {
        self.conditions.iter().any(|condition| {
            condition.kind == "Progressing"
                && condition.status == "False"
                && condition.reason == "ProgressDeadlineExceeded"
        })
    }

    fn is(&self, kind: &str, status: &str) -> bool {
        self.conditions
            .iter()
            .any(|condition| condition.kind == kind && condition.status == status)
    }
}

pub fn parse_deployment(json: &str) -> Result<Rollout, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|err| format!("unexpected kubectl output for the deployment: {err}"))?;
    let image = value
        .pointer("/spec/template/spec/containers/0/image")
        .and_then(|v| v.as_str())
        .ok_or("the deployment has no container image")?
        .to_string();
    let count = |path: &str| value.pointer(path).and_then(|v| v.as_u64()).unwrap_or(0);
    Ok(Rollout {
        image,
        selector: selector(&value),
        desired: count("/spec/replicas"),
        updated: count("/status/updatedReplicas"),
        ready: count("/status/readyReplicas"),
        available: count("/status/availableReplicas"),
        conditions: value
            .pointer("/status/conditions")
            .and_then(|v| v.as_array())
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|condition| Condition {
                kind: text(condition, "type"),
                status: text(condition, "status"),
                reason: text(condition, "reason"),
                message: text(condition, "message"),
            })
            .collect(),
    })
}

fn selector(deployment: &serde_json::Value) -> String {
    let mut labels: Vec<String> = deployment
        .pointer("/spec/selector/matchLabels")
        .and_then(|v| v.as_object())
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| Some(format!("{key}={}", value.as_str()?)))
        .collect();
    labels.sort();
    labels.join(",")
}

fn text(node: &serde_json::Value, key: &str) -> String {
    node.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Waiting {
    pub reason: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LastExit {
    pub reason: String,
    pub exit_code: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pod {
    pub name: String,
    pub phase: String,
    pub ready: bool,
    pub restarts: u64,
    pub started_at: Option<SystemTime>,
    pub sha: Option<String>,
    pub waiting: Option<Waiting>,
    pub stuck: Option<String>,
    pub last_exit: Option<LastExit>,
    pub terminating: bool,
}

pub fn parse_pods(json: &str) -> Result<Vec<Pod>, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|err| format!("unexpected kubectl output for the pods: {err}"))?;
    let mut pods: Vec<Pod> = value
        .get("items")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(parse_pod)
        .collect();
    pods.sort_by_key(|pod| match (&pod.stuck, pod.ready) {
        (Some(_), _) => 0,
        (None, false) => 1,
        (None, true) => 2,
    });
    Ok(pods)
}

fn waiting_reason(container: &serde_json::Value) -> Option<&str> {
    container
        .pointer("/state/waiting/reason")
        .and_then(|v| v.as_str())
}

fn restarting(container: &serde_json::Value) -> bool {
    container.get("ready").and_then(|v| v.as_bool()) != Some(true)
        && container
            .get("restartCount")
            .and_then(|v| v.as_u64())
            .is_some_and(|restarts| restarts > 0)
        && container.pointer("/lastState/terminated").is_some()
}

fn parse_pod(pod: &serde_json::Value) -> Pod {
    let containers: Vec<&serde_json::Value> = ["initContainerStatuses", "containerStatuses"]
        .iter()
        .filter_map(|key| pod.pointer(&format!("/status/{key}"))?.as_array())
        .flatten()
        .collect();
    let waiting = containers
        .iter()
        .find(|container| waiting_reason(container).is_some_and(|reason| STUCK.contains(&reason)))
        .or_else(|| {
            containers
                .iter()
                .find(|container| waiting_reason(container).is_some())
        });
    let restarting = containers.iter().find(|container| restarting(container));
    let stuck = waiting
        .and_then(|container| waiting_reason(container))
        .filter(|reason| STUCK.contains(reason))
        .map(str::to_string)
        .or_else(|| restarting.map(|_| "restarting".to_string()));
    Pod {
        name: pod
            .pointer("/metadata/name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        phase: pod
            .pointer("/status/phase")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        ready: pod
            .pointer("/status/containerStatuses")
            .and_then(|v| v.as_array())
            .is_some_and(|all| {
                !all.is_empty()
                    && all
                        .iter()
                        .all(|c| c.get("ready").and_then(|v| v.as_bool()) == Some(true))
            }),
        restarts: containers
            .iter()
            .filter_map(|container| container.get("restartCount")?.as_u64())
            .sum(),
        started_at: pod
            .pointer("/status/startTime")
            .and_then(|v| v.as_str())
            .and_then(parse_timestamp),
        sha: pod
            .pointer("/spec/containers/0/image")
            .and_then(|v| v.as_str())
            .and_then(|image| sha_from_image(image).ok()),
        waiting: waiting.and_then(|container| {
            let waiting = container.pointer("/state/waiting")?;
            Some(Waiting {
                reason: text(waiting, "reason"),
                message: text(waiting, "message"),
            })
        }),
        stuck,
        last_exit: waiting.or(restarting).and_then(|container| {
            let terminated = container.pointer("/lastState/terminated")?;
            Some(LastExit {
                reason: text(terminated, "reason"),
                exit_code: terminated.get("exitCode")?.as_i64()?,
            })
        }),
        terminating: pod.pointer("/metadata/deletionTimestamp").is_some(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Health {
    #[default]
    Unknown,
    Healthy,
    Unread,
    Warning(String),
    Alert(String),
}

const STUCK: [&str; 4] = [
    "CrashLoopBackOff",
    "ImagePullBackOff",
    "ErrImagePull",
    "CreateContainerConfigError",
];

pub fn health(rollout: &Rollout, pods: Option<&[Pod]>) -> Health {
    if !rollout.is("Available", "True") {
        return Health::Alert(format!(
            "{} of {} available",
            rollout.available, rollout.desired
        ));
    }
    let live: Vec<&Pod> = pods
        .unwrap_or_default()
        .iter()
        .filter(|pod| !pod.terminating && !matches!(pod.phase.as_str(), "Failed" | "Succeeded"))
        .collect();
    let stuck: Vec<&str> = live.iter().filter_map(|pod| pod.stuck.as_deref()).collect();
    if let Some(first) = stuck.first() {
        let reason = if stuck.iter().all(|reason| reason == first) {
            first
        } else {
            "stuck"
        };
        return Health::Warning(format!("{} of {} pods {reason}", stuck.len(), live.len()));
    }
    if rollout.stalled() {
        return Health::Warning("rollout stalled".to_string());
    }
    match pods {
        Some(_) => Health::Healthy,
        None => Health::Unread,
    }
}
