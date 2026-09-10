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
