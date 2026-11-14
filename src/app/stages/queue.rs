use std::time::SystemTime;

use crate::app::{Column, Listing, Row};
use crate::model::builds::Build;
use crate::model::queue::QueueEntry;

use super::{Context, Rows, StageView};

#[derive(Debug, Clone, Default)]
pub struct QueueStage {
    pub column: Column<QueueEntry>,
    pub repo: Option<String>,
}

impl StageView for QueueStage {
    fn listing(&self) -> &dyn Listing {
        &self.column
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        &mut self.column
    }

    fn receive(&mut self, _: usize, rows: Rows, now: SystemTime) {
        if let Rows::Queue(queue) = rows {
            self.repo = Some(queue.repo);
            self.column.receive(queue.entries, now);
        }
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        self.column
            .selected()
            .map(|entry| (entry.key(), entry.url.clone()))
            .ok_or_else(|| "nothing selected".to_string())
    }

    fn build_url(&self, _: &Context) -> Result<String, String> {
        self.column
            .selected()
            .and_then(|entry| entry.run.as_ref())
            .map(|run| run.url.clone())
            .ok_or_else(|| "no merge-group run yet".to_string())
    }

    fn selected_run(&self) -> Option<&Build> {
        self.column.selected()?.run.as_ref()
    }

    fn runs(&self) -> Vec<&Build> {
        self.column
            .all()
            .iter()
            .filter_map(|entry| entry.run.as_ref())
            .collect()
    }
}
