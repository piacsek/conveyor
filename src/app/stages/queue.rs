use std::time::SystemTime;

use crate::app::{Column, Listing, Row};
use crate::model::builds::Build;
use crate::model::queue::QueueEntry;

use super::{Context, Rows, StageView};

#[derive(Debug, Clone)]
pub struct QueueStage {
    names: Vec<String>,
    tabs: Vec<QueueTab>,
    active: usize,
}

#[derive(Debug, Clone, Default)]
struct QueueTab {
    column: Column<QueueEntry>,
    repo: Option<String>,
    absent: bool,
}

impl QueueStage {
    pub fn new(names: Vec<String>) -> Self {
        Self {
            tabs: names.iter().map(|_| QueueTab::default()).collect(),
            names,
            active: 0,
        }
    }

    pub fn column(&self) -> &Column<QueueEntry> {
        &self.tabs[self.active].column
    }

    pub fn column_mut(&mut self) -> &mut Column<QueueEntry> {
        &mut self.tabs[self.active].column
    }

    pub fn repo(&self) -> Option<&str> {
        self.tabs[self.active].repo.as_deref()
    }

    pub fn absent(&self) -> bool {
        self.tabs[self.active].absent
    }
}

impl StageView for QueueStage {
    fn listing(&self) -> &dyn Listing {
        self.column()
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        self.column_mut()
    }

    fn receive(&mut self, tab: usize, rows: Rows, now: SystemTime) {
        if let (Rows::Queue(queue), Some(tab)) = (rows, self.tabs.get_mut(tab)) {
            tab.repo = Some(queue.repo);
            tab.absent = queue.absent;
            tab.column.receive(queue.entries, now);
        }
    }

    fn listing_in_mut(&mut self, tab: usize) -> Option<&mut dyn Listing> {
        self.tabs
            .get_mut(tab)
            .map(|tab| &mut tab.column as &mut dyn Listing)
    }

    fn holds(&self, key: u64) -> bool {
        self.tabs.iter().any(|tab| tab.column.holds(key))
    }

    fn tab_names(&self) -> &[String] {
        &self.names
    }

    fn active_tab(&self) -> usize {
        self.active
    }

    fn tab_struck(&self, tab: usize) -> bool {
        self.tabs.get(tab).is_some_and(|tab| tab.absent)
    }

    fn select_tab(&mut self, tab: usize) {
        self.column_mut().clear_filter();
        self.active = tab;
        self.column_mut().clear_filter();
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        self.column()
            .selected()
            .map(|entry| (entry.key(), entry.url.clone()))
            .ok_or_else(|| "nothing selected".to_string())
    }

    fn build_url(&self, _: &Context) -> Result<String, String> {
        self.column()
            .selected()
            .and_then(|entry| entry.run.as_ref())
            .map(|run| run.url.clone())
            .ok_or_else(|| "no merge-group run yet".to_string())
    }

    fn selected_run(&self) -> Option<&Build> {
        self.column().selected()?.run.as_ref()
    }

    fn runs(&self) -> Vec<&Build> {
        self.runs_in(self.active)
    }

    fn runs_in(&self, tab: usize) -> Vec<&Build> {
        self.tabs
            .get(tab)
            .map_or(&self.tabs[self.active].column, |tab| &tab.column)
            .all()
            .iter()
            .filter_map(|entry| entry.run.as_ref())
            .collect()
    }
}
