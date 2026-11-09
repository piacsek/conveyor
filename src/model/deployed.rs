use std::time::SystemTime;

use crate::model::builds::PullRef;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deployment {
    pub env: String,
    pub image: Option<String>,
    pub sha: Option<String>,
    pub pull: Option<PullRef>,
    pub error: Option<String>,
    pub fetched_at: Option<SystemTime>,
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
