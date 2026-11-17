use super::Config;

const MIN_REFRESH_SECS: u64 = 10;

pub(super) fn check(config: &Config) -> Result<(), String> {
    refresh("[prs] refresh_secs", config.prs.refresh_secs)?;
    for repo in &config.repo {
        refresh(
            &format!("[[repo]] refresh_secs of {}", repo.name),
            repo.refresh_secs,
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
