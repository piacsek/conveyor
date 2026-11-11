use std::time::SystemTime;

use crate::app::{Column, Listing};
use crate::model::deployed::Deployment;

use super::{Context, Rows, StageView};

#[derive(Debug, Clone, Default)]
pub struct DeployedStage {
    pub column: Column<Deployment>,
    pub system: Option<String>,
}

impl StageView for DeployedStage {
    fn listing(&self) -> &dyn Listing {
        &self.column
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        &mut self.column
    }

    fn receive(&mut self, rows: Rows, now: SystemTime) {
        if let Rows::Deployed(deployed) = rows {
            self.system = Some(deployed.system);
            let rows = keep_last_known(self.column.all(), deployed.rows);
            self.column.receive(rows, now);
        }
    }

    fn error(&self) -> Option<&str> {
        self.column
            .error
            .as_deref()
            .or_else(|| self.column.selected().and_then(|row| row.error.as_deref()))
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        self.column
            .selected()
            .ok_or_else(|| "nothing selected".to_string())?
            .pull
            .as_ref()
            .map(|pull| (pull.number, pull.url.clone()))
            .ok_or_else(|| "no pull request for this commit".to_string())
    }

    fn build_url(&self, context: &Context) -> Result<String, String> {
        let sha = self
            .column
            .selected()
            .and_then(|row| row.sha.clone())
            .ok_or_else(|| "no deployed sha to look up".to_string())?;
        match context.main_build(&sha) {
            Some(build) => Ok(build.url.clone()),
            // Older than the fetched main builds: the commit page still shows its
            // checks and the run behind them.
            None => context
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

fn keep_last_known(current: &[Deployment], fresh: Vec<Deployment>) -> Vec<Deployment> {
    fresh
        .into_iter()
        .map(|mut row| {
            if row.sha.is_none()
                && let Some(known) = current.iter().find(|old| old.env == row.env)
            {
                row.sha = known.sha.clone();
                row.image = known.image.clone();
                row.pull = known.pull.clone();
                row.fetched_at = known.fetched_at;
            }
            row
        })
        .collect()
}
