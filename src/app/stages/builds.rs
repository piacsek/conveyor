use std::time::SystemTime;

use crate::app::actions::rerun_offer;
use crate::app::{Column, Listing, Menu, Rerun};
use crate::model::builds::{Build, BuildStatus};

use super::{Context, Rows, StageView};

#[derive(Debug, Clone, Default)]
pub struct BuildsStage {
    pub column: Column<Build>,
    pub repo: Option<String>,
}

impl StageView for BuildsStage {
    fn listing(&self) -> &dyn Listing {
        &self.column
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        &mut self.column
    }

    fn receive(&mut self, rows: Rows, now: SystemTime) {
        if let Rows::Builds(builds) = rows {
            self.repo = Some(builds.repo);
            self.column.receive(builds.builds, now);
        }
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        let build = self
            .column
            .selected()
            .ok_or_else(|| "nothing selected".to_string())?;
        match (&build.pull, build.pr_number, &self.repo) {
            (Some(pull), _, _) => Ok((pull.number, pull.url.clone())),
            (None, Some(number), Some(repo)) => {
                Ok((number, format!("https://github.com/{repo}/pull/{number}")))
            }
            _ => Err("no pull request for this run".to_string()),
        }
    }

    fn build_url(&self, _: &Context) -> Result<String, String> {
        self.column
            .selected()
            .map(|build| build.url.clone())
            .ok_or_else(|| "nothing selected".to_string())
    }

    fn selected_run(&self) -> Option<&Build> {
        self.column.selected()
    }

    fn runs(&self) -> Vec<&Build> {
        self.column.all().iter().collect()
    }

    fn menu(&self, _: &Context) -> Result<Menu, String> {
        if self.column.error.is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        let build = self.column.selected().ok_or("nothing selected")?;
        Ok(Menu {
            title: format!("Manage run {}", build.run_number),
            offers: vec![rerun_offer(self.rerun_target())],
        })
    }

    fn changed(&self) -> String {
        "the run changed; press S again".to_string()
    }
}

impl BuildsStage {
    fn rerun_target(&self) -> Result<Rerun, String> {
        let build = self.column.selected().ok_or("nothing selected")?;
        let label = format!("run {}", build.run_number);
        let newest = self.column.all().iter().map(|b| b.run_number).max();
        if newest != Some(build.run_number) {
            return Err("only the newest main build can be rerun".to_string());
        }
        if !build.is_settled() {
            return Err(format!("{label} is still running"));
        }
        if !matches!(build.status, BuildStatus::Failure | BuildStatus::Cancelled) {
            return Err("nothing failed to rerun".to_string());
        }
        let repo = self.repo.clone().ok_or("no repository known")?;
        Ok(Rerun {
            repo,
            run_ids: vec![build.id],
            label,
        })
    }
}
