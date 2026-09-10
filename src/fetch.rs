use crate::config::Config;
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
