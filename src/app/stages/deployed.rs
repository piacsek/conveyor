use std::time::SystemTime;

use crate::app::{Column, Listing};
use crate::model::deployed::Deployment;
use crate::sources::failure::Failure;

use super::{Context, Rows, StageView};

#[derive(Debug, Clone)]
pub struct DeployedStage {
    names: Vec<String>,
    tabs: Vec<DeployedTab>,
    active: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DeployedTab {
    pub column: Column<Deployment>,
    pub system: Option<String>,
}

impl DeployedStage {
    pub fn new(names: Vec<String>) -> Self {
        Self {
            tabs: vec![DeployedTab::default(); names.len().max(1)],
            names,
            active: 0,
        }
    }

    pub fn active(&self) -> &DeployedTab {
        &self.tabs[self.active]
    }

    pub fn tab(&self, tab: usize) -> &DeployedTab {
        self.tabs.get(tab).unwrap_or(self.active())
    }

    pub fn column_mut(&mut self) -> &mut Column<Deployment> {
        &mut self.tabs[self.active].column
    }

    fn column(&self) -> &Column<Deployment> {
        &self.active().column
    }
}

impl StageView for DeployedStage {
    fn listing(&self) -> &dyn Listing {
        self.column()
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        self.column_mut()
    }

    fn receive(&mut self, tab: usize, rows: Rows, now: SystemTime) {
        if let (Rows::Deployed(deployed), Some(tab)) = (rows, self.tabs.get_mut(tab)) {
            tab.system = Some(deployed.system);
            let rows = keep_last_known(tab.column.all(), deployed.rows);
            tab.column.receive(rows, now);
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

    fn select_tab(&mut self, tab: usize) {
        if tab < self.tabs.len() {
            self.column_mut().clear_filter();
            self.active = tab;
            self.column_mut().clear_filter();
        }
    }

    fn holds(&self, key: u64) -> bool {
        self.tabs.iter().any(|tab| tab.column.holds(key))
    }

    fn error(&self) -> Option<&str> {
        self.column()
            .error
            .as_ref()
            .map(Failure::message)
            .or_else(|| {
                self.column()
                    .selected()
                    .and_then(|row| row.error.as_ref().map(Failure::message))
            })
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        self.column()
            .selected()
            .ok_or_else(|| "nothing selected".to_string())?
            .pull
            .as_ref()
            .map(|pull| (pull.number, pull.url.clone()))
            .ok_or_else(|| "no pull request for this commit".to_string())
    }

    fn build_url(&self, context: &Context) -> Result<String, String> {
        let sha = self
            .column()
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
