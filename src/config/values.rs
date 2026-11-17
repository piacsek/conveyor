use std::fmt::Display;
use std::ops::RangeInclusive;

use super::{Config, DeployEnv, Prs, Repo, Ui};

const REFRESH_SECS: RangeInclusive<u64> = 10..=86_400;
const PAGE: RangeInclusive<usize> = 1..=100;
const DETAILS_PERCENT: RangeInclusive<u16> = 10..=90;
const MAX_COLUMN_WIDTH: u16 = u16::MAX / 4;

pub(super) fn check(config: &Config) -> Result<(), String> {
    check_prs(&config.prs)?;
    check_ui(&config.ui)?;
    config.repo.iter().try_for_each(check_repo)?;
    if let Some(repo) = repeated(&config.repo, |a, b| a.name.eq_ignore_ascii_case(&b.name)) {
        return Err(format!("two [[repo]] are named \"{}\"", repo.name));
    }
    check_read(&config.repo)
}

fn check_read(repos: &[Repo]) -> Result<(), String> {
    let mut deployed = repos.iter().filter(|repo| !repo.deploy.is_empty());
    if let (Some(read), Some(ignored)) = (deployed.next(), deployed.next()) {
        return Err(format!(
            "the [[repo.deploy]] of {} is ignored: conveyor reads only the one of {}",
            ignored.name, read.name
        ));
    }
    Ok(())
}

fn check_prs(prs: &Prs) -> Result<(), String> {
    within("[prs] refresh_secs", prs.refresh_secs, &REFRESH_SECS)?;
    within("[prs] limit", prs.limit, &PAGE)?;
    for tab in &prs.tab {
        if let Some(limit) = tab.limit {
            within(
                &format!("[[prs.tab]] limit of \"{}\"", tab.name),
                limit,
                &PAGE,
            )?;
        }
    }
    Ok(())
}

fn check_ui(ui: &Ui) -> Result<(), String> {
    within("[ui] details_percent", ui.details_percent, &DETAILS_PERCENT)?;
    if ui.min_column_width > MAX_COLUMN_WIDTH {
        return Err(format!(
            "[ui] min_column_width is {}; use {MAX_COLUMN_WIDTH} or less",
            ui.min_column_width
        ));
    }
    Ok(())
}

fn check_repo(repo: &Repo) -> Result<(), String> {
    if !owner_and_name(&repo.name) {
        return Err(format!("[[repo]] name \"{}\" is not owner/name", repo.name));
    }
    within(
        &format!("[[repo]] refresh_secs of {}", repo.name),
        repo.refresh_secs,
        &REFRESH_SECS,
    )?;
    within(
        &format!("[[repo]] builds of {}", repo.name),
        repo.builds,
        &PAGE,
    )?;
    if repo.deploy.len() > 1 {
        return Err(format!(
            "{} has a second [[repo.deploy]]; conveyor reads only the first",
            repo.name
        ));
    }
    for deploy in &repo.deploy {
        within(
            &format!("[[repo.deploy]] refresh_secs of {}", repo.name),
            deploy.refresh_secs,
            &REFRESH_SECS,
        )?;
        for env in &deploy.env {
            check_env(env, &repo.name)?;
        }
        if let Some(env) = repeated(&deploy.env, |a, b| a.name == b.name) {
            return Err(format!(
                "two [[repo.deploy.env]] of {} are named \"{}\"",
                repo.name, env.name
            ));
        }
    }
    Ok(())
}

fn check_env(env: &DeployEnv, repo: &str) -> Result<(), String> {
    if env.name.trim().is_empty() {
        return Err(format!("a [[repo.deploy.env]] of {repo} has a blank name"));
    }
    if env.deployment.trim().is_empty() {
        return Err(format!(
            "[[repo.deploy.env]] \"{}\" of {repo} has a blank deployment",
            env.name
        ));
    }
    Ok(())
}

fn repeated<T>(items: &[T], same: impl Fn(&T, &T) -> bool) -> Option<&T> {
    items
        .iter()
        .enumerate()
        .find(|(i, item)| items[..*i].iter().any(|other| same(other, item)))
        .map(|(_, item)| item)
}

fn owner_and_name(name: &str) -> bool {
    let part = |part: &str| {
        !part.is_empty()
            && !part.contains('/')
            && !part.chars().any(|c| c.is_whitespace() || c.is_control())
    };
    name.split_once('/')
        .is_some_and(|(owner, name)| part(owner) && part(name))
}

fn within<T: PartialOrd + Display>(
    what: &str,
    value: T,
    range: &RangeInclusive<T>,
) -> Result<(), String> {
    if !range.contains(&value) {
        return Err(format!(
            "{what} is {value}; use {} to {}",
            range.start(),
            range.end()
        ));
    }
    Ok(())
}
