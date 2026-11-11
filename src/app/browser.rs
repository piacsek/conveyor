use std::time::Duration;

use super::{Action, App, Row, Stage};

pub const OPEN_DEBOUNCE: Duration = Duration::from_secs(1);

impl App {
    pub(super) fn pull_target(&self) -> Result<(u64, String), String> {
        let nothing = || "nothing selected".to_string();
        match self.focus {
            Stage::Prs => self
                .prs
                .column
                .selected()
                .map(|pr| (pr.key(), pr.url.clone()))
                .ok_or_else(nothing),
            Stage::Queue => self
                .queue
                .column
                .selected()
                .map(|entry| (entry.key(), entry.url.clone()))
                .ok_or_else(nothing),
            Stage::Builds => {
                let build = self.builds.column.selected().ok_or_else(nothing)?;
                match (&build.pull, build.pr_number, &self.builds.repo) {
                    (Some(pull), _, _) => Ok((pull.number, pull.url.clone())),
                    (None, Some(number), Some(repo)) => {
                        Ok((number, format!("https://github.com/{repo}/pull/{number}")))
                    }
                    _ => Err("no pull request for this run".to_string()),
                }
            }
            Stage::Deployed => self
                .deployed
                .column
                .selected()
                .ok_or_else(nothing)?
                .pull
                .as_ref()
                .map(|pull| (pull.number, pull.url.clone()))
                .ok_or_else(|| "no pull request for this commit".to_string()),
        }
    }

    pub(super) fn build_url(&self) -> Result<String, String> {
        match self.focus {
            Stage::Prs => self
                .prs
                .column
                .selected()
                .and_then(|pr| {
                    pr.checks_failures_first()
                        .into_iter()
                        .find(|check| !check.url.is_empty())
                        .map(|check| check.url.clone())
                })
                .ok_or_else(|| "no checks yet".to_string()),
            Stage::Queue => self
                .queue
                .column
                .selected()
                .and_then(|entry| entry.run.as_ref())
                .map(|run| run.url.clone())
                .ok_or_else(|| "no merge-group run yet".to_string()),
            Stage::Builds => self
                .builds
                .column
                .selected()
                .map(|build| build.url.clone())
                .ok_or_else(|| "nothing selected".to_string()),
            Stage::Deployed => {
                let sha = self
                    .deployed
                    .column
                    .selected()
                    .and_then(|row| row.sha.clone())
                    .ok_or_else(|| "no deployed sha to look up".to_string())?;
                match self
                    .builds
                    .column
                    .all()
                    .iter()
                    .find(|build| build.sha == sha)
                {
                    Some(build) => Ok(build.url.clone()),
                    // Older than the fetched main builds: the commit page still shows its
                    // checks and the run behind them.
                    None => self
                        .deploy_repo()
                        .map(|repo| format!("https://github.com/{repo}/commit/{sha}"))
                        .ok_or_else(|| {
                            format!(
                                "no main build found for {}",
                                sha.chars().take(8).collect::<String>()
                            )
                        }),
                }
            }
        }
    }

    /// The repository the Deployed column watches: the configured one with a `[[repo.deploy]]`,
    /// else the one the main builds came from.
    fn deploy_repo(&self) -> Option<String> {
        self.config
            .repo
            .iter()
            .find(|repo| !repo.deploy.is_empty())
            .map(|repo| repo.name.clone())
            .or_else(|| self.builds.repo.clone())
    }

    pub(super) fn open(&mut self, url: String) -> Action {
        if let Err(err) = crate::open::web_url(&url) {
            self.notice = Some(err.to_string());
            return Action::Continue;
        }
        if self.opened_recently(&url) {
            return Action::Continue;
        }
        self.last_open = Some((url.clone(), self.now));
        Action::Open(url)
    }

    fn opened_recently(&self, url: &str) -> bool {
        self.last_open.as_ref().is_some_and(|(last, at)| {
            last == url
                && self
                    .now
                    .duration_since(*at)
                    .is_ok_and(|elapsed| elapsed < OPEN_DEBOUNCE)
        })
    }
}
