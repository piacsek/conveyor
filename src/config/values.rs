use std::ops::RangeInclusive;

use super::Config;

const MIN_REFRESH_SECS: u64 = 10;
const PAGE: RangeInclusive<usize> = 1..=100;

pub(super) fn check(config: &Config) -> Result<(), String> {
    refresh("[prs] refresh_secs", config.prs.refresh_secs)?;
    within("[prs] limit", config.prs.limit, &PAGE)?;
    for tab in &config.prs.tab {
        if let Some(limit) = tab.limit {
            within(
                &format!("[[prs.tab]] limit of \"{}\"", tab.name),
                limit,
                &PAGE,
            )?;
        }
    }
    for repo in &config.repo {
        refresh(
            &format!("[[repo]] refresh_secs of {}", repo.name),
            repo.refresh_secs,
        )?;
        within(
            &format!("[[repo]] builds of {}", repo.name),
            repo.builds,
            &PAGE,
        )?;
        for deploy in &repo.deploy {
            refresh(
                &format!("[[repo.deploy]] refresh_secs of {}", repo.name),
                deploy.refresh_secs,
            )?;
        }
    }
    Ok(())
}

fn refresh(what: &str, secs: u64) -> Result<(), String> {
    if secs < MIN_REFRESH_SECS {
        return Err(format!("{what} is {secs}; use {MIN_REFRESH_SECS} or more"));
    }
    Ok(())
}

fn within(what: &str, value: usize, range: &RangeInclusive<usize>) -> Result<(), String> {
    if !range.contains(&value) {
        return Err(format!(
            "{what} is {value}; use {} to {}",
            range.start(),
            range.end()
        ));
    }
    Ok(())
}
