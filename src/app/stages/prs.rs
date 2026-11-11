use std::time::SystemTime;

use crate::app::{Column, Listing, Row};
use crate::model::prs::PullRequest;

use super::{Context, Rows, StageView};

#[derive(Debug, Clone, Default)]
pub struct PrsStage {
    pub column: Column<PullRequest>,
}

impl StageView for PrsStage {
    fn listing(&self) -> &dyn Listing {
        &self.column
    }

    fn listing_mut(&mut self) -> &mut dyn Listing {
        &mut self.column
    }

    fn receive(&mut self, rows: Rows, now: SystemTime) {
        if let Rows::Prs(prs) = rows {
            self.column.receive(prs, now);
        }
    }

    fn pull_target(&self) -> Result<(u64, String), String> {
        self.column
            .selected()
            .map(|pr| (pr.key(), pr.url.clone()))
            .ok_or_else(|| "nothing selected".to_string())
    }

    fn build_url(&self, _: &Context) -> Result<String, String> {
        self.column
            .selected()
            .and_then(|pr| {
                pr.checks_failures_first()
                    .into_iter()
                    .find(|check| !check.url.is_empty())
                    .map(|check| check.url.clone())
            })
            .ok_or_else(|| "no checks yet".to_string())
    }
}
