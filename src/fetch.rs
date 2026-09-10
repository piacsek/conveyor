use crate::config::{Config, Repo};
use crate::github::Github;
use crate::model::prs::{PullRequest, parse};

pub const PRS_QUERY: &str = include_str!("queries/prs.graphql");

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
    parse(&value)
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
