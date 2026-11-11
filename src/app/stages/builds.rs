use std::time::SystemTime;

use crate::app::{Column, Listing};
use crate::model::builds::Build;

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
}
