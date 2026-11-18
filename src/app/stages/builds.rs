use std::time::SystemTime;

use crate::app::actions::rerun_offer;
use crate::app::{Column, Listing, Menu, Rerun, Row};
use crate::model::builds::{Build, BuildStatus};

use super::{Context, Rows, StageView};

#[derive(Debug, Clone)]
pub struct BuildsStage {
    names: Vec<String>,
    tabs: Vec<BuildsTab>,
    active: usize,
}

#[derive(Debug, Clone, Default)]
pub struct BuildsTab {
    pub column: Column<Build>,
    pub repo: Option<String>,
    pub branch: String,
}

impl BuildsStage {
    pub fn new(names: Vec<String>) -> Self {
        Self {
            tabs: vec![BuildsTab::default(); names.len().max(1)],
            names,
            active: 0,
        }
    }

    pub fn active(&self) -> &BuildsTab {
        &self.tabs[self.active]
    }

    pub fn tab(&self, tab: usize) -> &BuildsTab {
        self.tabs.get(tab).unwrap_or(self.active())
    }

    pub fn column_mut(&mut self) -> &mut Column<Build> {
        &mut self.tabs[self.active].column
    }

    pub fn follow(&mut self, tab: usize) {
        if tab < self.tabs.len() {
            self.column_mut().clear_filter();
            self.active = tab;
            self.column_mut().clear_filter();
        }
    }
}

impl StageView for BuildsStage {
    fn listing(&self) -> &dyn Listing {
        &self.active().column
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        self.column_mut()
    }

    fn receive(&mut self, tab: usize, rows: Rows, now: SystemTime) {
        if let (Rows::Builds(builds), Some(tab)) = (rows, self.tabs.get_mut(tab)) {
            tab.repo = Some(builds.repo);
            tab.branch = builds.branch;
            tab.column.receive(builds.builds, now);
        }
    }

    fn listing_in_mut(&mut self, tab: usize) -> Option<&mut dyn Listing> {
        self.tabs
            .get_mut(tab)
            .map(|tab| &mut tab.column as &mut dyn Listing)
    }

    fn tab_names(&self) -> &[String] {
        &self.names
    }

    fn active_tab(&self) -> usize {
        self.active
    }

    fn holds(&self, key: u64) -> bool {
        self.tabs.iter().any(|tab| tab.column.holds(key))
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        self.active()
            .column
            .selected()
            .ok_or_else(|| "nothing selected".to_string())?
            .pull_request(self.active().repo.as_deref())
            .ok_or_else(|| "no pull request for this run".to_string())
    }

    fn build_url(&self, _: &Context) -> Result<String, String> {
        self.active()
            .column
            .selected()
            .map(|build| build.url.clone())
            .ok_or_else(|| "nothing selected".to_string())
    }

    fn selected_run(&self) -> Option<&Build> {
        self.active().column.selected()
    }

    fn runs(&self) -> Vec<&Build> {
        self.runs_in(self.active)
    }

    fn runs_in(&self, tab: usize) -> Vec<&Build> {
        self.tab(tab).column.all().iter().collect()
    }

    fn menu(&self, _: &Context) -> Result<Menu, String> {
        if self.active().column.error.is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        let build = self.active().column.selected().ok_or("nothing selected")?;
        Ok(Menu {
            subject: build.key(),
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
        let build = self.active().column.selected().ok_or("nothing selected")?;
        let label = format!("run {}", build.run_number);
        let newest = self
            .active()
            .column
            .all()
            .iter()
            .map(|b| b.run_number)
            .max();
        if newest != Some(build.run_number) {
            return Err("only the newest main build can be rerun".to_string());
        }
        if !build.is_settled() {
            return Err(format!("{label} is still running"));
        }
        if !matches!(build.status, BuildStatus::Failure | BuildStatus::Cancelled) {
            return Err("nothing failed to rerun".to_string());
        }
        let repo = self.active().repo.clone().ok_or("no repository known")?;
        Ok(Rerun {
            repo,
            run_ids: vec![build.id],
            label,
        })
    }
}
