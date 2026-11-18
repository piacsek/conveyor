use std::time::SystemTime;

use crate::app::actions::offer;
use crate::app::{Choice, Column, Listing, Menu, Offer, Row, StatusTarget};
use crate::model::builds::Build;
use crate::model::prs::StatusChange;
use crate::model::queue::{QueueEntry, QueueState};

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
        if let (Rows::Queue(mut queue), Some(tab)) = (rows, self.tabs.get_mut(tab)) {
            for entry in &mut queue.entries {
                entry.repo.clone_from(&queue.repo);
            }
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
            .map(|entry| (entry.number, entry.url.clone()))
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

    fn menu(&self, _: &Context) -> Result<Menu, String> {
        if self.column().error.is_some() {
            return Err("the last refresh failed; press r and try again".to_string());
        }
        let entry = self.column().selected().ok_or("nothing selected")?;
        Ok(Menu {
            subject: entry.key(),
            title: format!("Manage #{}", entry.number),
            offers: offers(entry),
        })
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

fn offers(entry: &QueueEntry) -> Vec<Offer> {
    let status = |change: Result<StatusChange, &str>| {
        if entry.state == QueueState::Locked {
            return Err(format!("#{} is being merged", entry.number));
        }
        if entry.node_id.is_empty() || entry.head_oid.is_empty() {
            return Err(format!("GitHub sent no id for #{}", entry.number));
        }
        Ok(Choice::Status(StatusTarget {
            number: entry.number,
            repo: entry.repo.clone(),
            title: entry.title.clone(),
            head_oid: entry.head_oid.clone(),
            change: change?,
        }))
    };
    let jump = match (entry.position, entry.jump) {
        (1, _) => Err("already at the front"),
        (_, true) => Err("already jumping the queue"),
        _ => Ok(StatusChange::Enqueue {
            node_id: entry.node_id.clone(),
            head_oid: entry.head_oid.clone(),
            jump: true,
        }),
    };
    vec![
        offer(
            'e',
            "remove from the merge queue",
            status(Ok(StatusChange::Dequeue {
                node_id: entry.node_id.clone(),
            })),
        ),
        offer('i', "move to the front", status(jump)),
    ]
}
