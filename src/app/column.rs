use std::time::SystemTime;

use ratatui::widgets::ListState;

use crate::model::builds::Build;
use crate::model::deployed::Deployment;
use crate::model::prs::PullRequest;
use crate::model::queue::QueueEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    AsFetched,
    ByRankAscending,
    ByRankDescending,
}

pub trait Row: Clone {
    const ORDER: Order = Order::AsFetched;

    fn key(&self) -> u64;

    fn rank(&self) -> u64 {
        0
    }

    fn matches(&self, query: &str) -> bool;
}

impl Row for PullRequest {
    const ORDER: Order = Order::ByRankAscending;

    fn key(&self) -> u64 {
        self.number
    }

    fn rank(&self) -> u64 {
        match (self.queue_position, self.is_draft) {
            (Some(position), _) => position,
            (None, false) => u64::MAX - 1,
            (None, true) => u64::MAX,
        }
    }

    fn matches(&self, query: &str) -> bool {
        [
            format!("#{}", self.number),
            self.title.to_lowercase(),
            self.repo.to_lowercase(),
        ]
        .iter()
        .any(|text| text.contains(query))
    }
}

impl Row for QueueEntry {
    const ORDER: Order = Order::ByRankAscending;

    fn key(&self) -> u64 {
        self.number
    }

    fn rank(&self) -> u64 {
        self.position
    }

    fn matches(&self, query: &str) -> bool {
        [
            format!("#{}", self.number),
            self.title.to_lowercase(),
            self.author.to_lowercase(),
        ]
        .iter()
        .any(|text| text.contains(query))
    }
}

impl Row for Build {
    const ORDER: Order = Order::ByRankDescending;

    fn key(&self) -> u64 {
        self.id
    }

    fn rank(&self) -> u64 {
        self.run_number
    }

    fn matches(&self, query: &str) -> bool {
        let pull = self.pull.as_ref();
        [
            self.pr_number
                .or(pull.map(|p| p.number))
                .map(|n| format!("#{n}"))
                .unwrap_or_default(),
            self.title.to_lowercase(),
            pull.map(|p| p.author.to_lowercase()).unwrap_or_default(),
            self.sha.clone(),
        ]
        .iter()
        .any(|text| text.contains(query))
    }
}

impl Row for Deployment {
    fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        self.env.hash(&mut hasher);
        hasher.finish()
    }

    fn matches(&self, query: &str) -> bool {
        let pull = self.pull.as_ref();
        [
            self.env.to_lowercase(),
            pull.map(|p| format!("#{}", p.number)).unwrap_or_default(),
            pull.map(|p| p.title.to_lowercase()).unwrap_or_default(),
            pull.map(|p| p.author.to_lowercase()).unwrap_or_default(),
            self.sha.clone().unwrap_or_default(),
        ]
        .iter()
        .any(|text| text.contains(query))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ColumnState<T> {
    #[default]
    Loading,
    Ready {
        rows: Vec<T>,
        fetched_at: SystemTime,
    },
}

impl<T> ColumnState<T> {
    pub fn into_rows(self) -> Vec<T> {
        match self {
            ColumnState::Ready { rows, .. } => rows,
            ColumnState::Loading => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column<T> {
    pub state: ColumnState<T>,
    pub error: Option<String>,
    pub refreshing: bool,
    pub list: ListState,
    pub filter: Option<String>,
}

impl<T> Default for Column<T> {
    fn default() -> Self {
        Self {
            state: ColumnState::Loading,
            error: None,
            refreshing: false,
            list: ListState::default().with_selected(Some(0)),
            filter: None,
        }
    }
}

impl<T: Row> Column<T> {
    pub fn all(&self) -> &[T] {
        match &self.state {
            ColumnState::Ready { rows, .. } => rows,
            ColumnState::Loading => &[],
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.state, ColumnState::Loading)
    }

    pub fn fetched_at(&self) -> Option<SystemTime> {
        match &self.state {
            ColumnState::Ready { fetched_at, .. } => Some(*fetched_at),
            ColumnState::Loading => None,
        }
    }

    pub fn visible(&self) -> Vec<&T> {
        let query = self.filter.clone().unwrap_or_default().to_lowercase();
        self.all()
            .iter()
            .filter(|row| row.matches(&query))
            .collect()
    }

    pub fn selected(&self) -> Option<&T> {
        self.list
            .selected()
            .and_then(|i| self.visible().get(i).copied())
    }

    pub fn receive(&mut self, rows: Vec<T>, now: SystemTime) {
        let selected = self.selected().map(Row::key);
        let current = std::mem::take(&mut self.state).into_rows();
        let mut rows = merge_keeping_order(current, rows);
        match T::ORDER {
            Order::AsFetched => {}
            Order::ByRankAscending => rows.sort_by_key(Row::rank),
            Order::ByRankDescending => rows.sort_by_key(|row| std::cmp::Reverse(row.rank())),
        }
        self.state = ColumnState::Ready {
            rows,
            fetched_at: now,
        };
        self.error = None;
        self.refreshing = false;
        let index = selected
            .and_then(|key| self.visible().iter().position(|row| row.key() == key))
            .unwrap_or(0);
        self.list.select(Some(index));
    }

    pub fn fail(&mut self, message: String) {
        self.error = Some(message);
        self.refreshing = false;
    }

    fn select_next(&mut self) {
        let last = self.visible().len().saturating_sub(1);
        let next = self.list.selected().map_or(0, |i| (i + 1).min(last));
        self.list.select(Some(next));
    }

    fn select_last(&mut self) {
        self.list.select(self.visible().len().checked_sub(1));
    }

    fn select_index(&mut self, index: usize) {
        if index < self.visible().len() {
            self.list.select(Some(index));
        }
    }
}

fn merge_keeping_order<T: Row>(current: Vec<T>, fresh: Vec<T>) -> Vec<T> {
    let mut fresh = fresh;
    let mut merged: Vec<T> = current
        .iter()
        .filter_map(|old| {
            let index = fresh.iter().position(|new| new.key() == old.key())?;
            Some(fresh.remove(index))
        })
        .collect();
    merged.extend(fresh);
    merged
}

pub trait Navigable {
    fn next(&mut self);
    fn previous(&mut self);
    fn first(&mut self);
    fn last(&mut self);
    fn select_index_clamped(&mut self, index: usize);
}

impl<T: Row> Navigable for Column<T> {
    fn next(&mut self) {
        self.select_next();
    }

    fn previous(&mut self) {
        self.list.select_previous();
    }

    fn first(&mut self) {
        self.list.select_first();
    }

    fn last(&mut self) {
        self.select_last();
    }

    fn select_index_clamped(&mut self, index: usize) {
        if index == 0 {
            self.list.select_first();
        } else {
            self.select_index(index);
        }
    }
}
