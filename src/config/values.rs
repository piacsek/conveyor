use std::fmt::Display;
use std::ops::RangeInclusive;

use super::{Config, Deploy, DeployEnv, Prs, Repo, Ui};

const MIN_REFRESH_SECS: u64 = 5;
const PAGE: RangeInclusive<usize> = 1..=100;
const DETAILS_PERCENT: RangeInclusive<u16> = 1..=99;
const MAX_COLUMN_WIDTH: u16 = u16::MAX / 4;

pub(super) fn check(config: &Config) -> Result<(), String> {
    check_prs(&config.prs)?;
    check_ui(&config.ui)?;
    config.repo.iter().try_for_each(check_repo)?;
    if let Some(repo) = repeated(&config.repo, |a, b| {
        a.name.eq_ignore_ascii_case(&b.name)
            && a.branch == b.branch
            && a.main_workflow == b.main_workflow
    }) {
        return Err(format!(
            "two [[repo]] named {:?} follow the same branch and main_workflow; nothing tells their tabs apart",
            repo.name
        ));
    }
    if let Some(repo) = repeated(&config.repo, |a, b| {
        a.name.eq_ignore_ascii_case(&b.name)
            && a.merge_method.is_some()
            && b.merge_method.is_some()
            && a.merge_method != b.merge_method
    }) {
        return Err(format!(
            "two [[repo]] named {:?} set different merge_method values; only one of them can be used, so set it on one",
            repo.name
        ));
    }
    check_systems(config)
}

fn check_systems(config: &Config) -> Result<(), String> {
    let deploys: Vec<(&Repo, &Deploy)> = config.deploys().collect();
    if deploys.len() < 2 {
        return Ok(());
    }
    if let Some((repo, _)) = deploys
        .iter()
        .find(|(_, deploy)| deploy.system.trim().is_empty())
    {
        return Err(format!(
            "a [[repo.deploy]] of {} has a blank system; with several deploys each is a tab and needs one",
            entry(&config.repo, repo)
        ));
    }
    if let Some((_, deploy)) = repeated(&deploys, |(_, a), (_, b)| {
        a.system.trim().eq_ignore_ascii_case(b.system.trim())
    }) {
        return Err(format!(
            "two [[repo.deploy]] have the system {:?}; each deploy is a tab and needs a system of its own",
            deploy.system
        ));
    }
    Ok(())
}

fn entry(repos: &[Repo], repo: &Repo) -> String {
    let named = |other: &&Repo| other.name.eq_ignore_ascii_case(&repo.name);
    if repos.iter().filter(named).count() < 2 {
        return repo.name.clone();
    }
    let number = 1 + repos
        .iter()
        .position(|other| std::ptr::eq(other, repo))
        .unwrap_or_default();
    format!("{} ([[repo]] number {number})", repo.name)
}

fn check_prs(prs: &Prs) -> Result<(), String> {
    refresh("[prs] refresh_secs", prs.refresh_secs)?;
    if prs.tab.is_empty() || prs.tab.iter().any(|tab| tab.limit.is_none()) {
        within("[prs] limit", prs.limit, &PAGE)?;
    }
    for tab in &prs.tab {
        if let Some(limit) = tab.limit {
            within(
                &format!("[[prs.tab]] limit of {:?}", tab.name),
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
        return Err(format!("[[repo]] name {:?} is not owner/name", repo.name));
    }
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
        if deploy.env.is_empty() {
            return Err(format!(
                "the [[repo.deploy]] of {} has no [[repo.deploy.env]]",
                repo.name
            ));
        }
        for env in &deploy.env {
            check_env(env, &repo.name)?;
        }
        if let Some(env) = repeated(&deploy.env, |a, b| a.name == b.name) {
            return Err(format!(
                "two [[repo.deploy.env]] of {} are named {:?}",
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
            "[[repo.deploy.env]] {:?} of {repo} has a blank deployment",
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

fn refresh(what: &str, secs: u64) -> Result<(), String> {
    if secs < MIN_REFRESH_SECS {
        return Err(format!("{what} is {secs}; use {MIN_REFRESH_SECS} or more"));
    }
    Ok(())
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
