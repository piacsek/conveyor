use crate::config::{Config, Repo};
use crate::github::Github;
use crate::model::prs::PullRequest;
use crate::model::queue::Queue;

pub const PRS_QUERY: &str = include_str!("queries/prs.graphql");
pub const QUEUE_QUERY: &str = include_str!("queries/queue.graphql");
const QUEUE_LIMIT: usize = 20;

pub fn fetch_prs(gh: &impl Github, config: &Config) -> Result<Vec<PullRequest>, String> {
    let value = gh
        .graphql(
            PRS_QUERY,
            &[
                ("q", config.prs.query.as_str().into()),
                ("first", config.prs.limit.into()),
            ],
        )
        .map_err(|err| err.to_string())?;
    crate::model::prs::parse(&value)
}

pub fn repos(gh: &impl Github, config: &Config) -> Result<Vec<Repo>, String> {
    if !config.repo.is_empty() {
        return Ok(config.repo.clone());
    }
    let name = gh.current_repo().map_err(|err| {
        format!("no [[repo]] configured and the current directory has none: {err}")
    })?;
    Ok(vec![Repo {
        name,
        ..Repo::default()
    }])
}

pub fn fetch_queue(gh: &impl Github, repo: &Repo) -> Result<Queue, String> {
    let (owner, name) = repo
        .name
        .split_once('/')
        .ok_or_else(|| format!("repo `{}` must be owner/name", repo.name))?;
    let value = gh
        .graphql(
            QUEUE_QUERY,
            &[
                ("owner", owner.into()),
                ("name", name.into()),
                ("first", QUEUE_LIMIT.into()),
            ],
        )
        .map_err(|err| err.to_string())?;
    let mut queue = crate::model::queue::parse(&value)?;
    queue.repo = repo.name.clone();
    Ok(queue)
}
