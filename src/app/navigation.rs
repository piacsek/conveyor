use std::time::SystemTime;

use super::{App, Mode, Navigable, Stage};

impl App {
    pub fn filter(&self) -> Option<&str> {
        self.query.as_deref()
    }

    pub fn editing_filter(&self) -> bool {
        matches!(self.mode, Mode::Filter(_))
    }

    pub fn focused_error(&self) -> Option<&str> {
        match self.focus {
            Stage::Prs => self.prs.error.as_deref(),
            Stage::Queue => self.queue.error.as_deref(),
            Stage::Builds => self.builds.error.as_deref(),
            Stage::Deployed => self.deployed.error.as_deref().or_else(|| {
                self.deployed
                    .selected()
                    .and_then(|row| row.error.as_deref())
            }),
        }
    }

    pub fn focused_fetched_at(&self) -> Option<SystemTime> {
        match self.focus {
            Stage::Prs => self.prs.fetched_at(),
            Stage::Queue => self.queue.fetched_at(),
            Stage::Builds => self.builds.fetched_at(),
            Stage::Deployed => self.deployed.fetched_at(),
        }
    }

    pub fn focused_counts(&self) -> (usize, usize) {
        match self.focus {
            Stage::Prs => (self.prs.visible().len(), self.prs.all().len()),
            Stage::Queue => (self.queue.visible().len(), self.queue.all().len()),
            Stage::Builds => (self.builds.visible().len(), self.builds.all().len()),
            Stage::Deployed => (self.deployed.visible().len(), self.deployed.all().len()),
        }
    }

    pub(super) fn refocus(&mut self, step: impl Fn(Stage) -> Stage) {
        self.focus = step(self.focus);
        self.reset_scroll();
    }

    /// Both panes start at their resting place: the details at the top, the log at its end.
    pub(super) fn reset_scroll(&mut self) {
        self.details_scroll = 0;
        self.log_scroll = u16::MAX;
    }

    /// `Ctrl-d`/`Ctrl-u` move the log pane while it is open, else the details pane.
    pub(super) fn scroll_details(&mut self, half_pages: i32) {
        let (scroll, page) = if self.details && self.selected_log().is_some() {
            (&mut self.log_scroll, self.log_page)
        } else {
            (&mut self.details_scroll, self.details_page)
        };
        let step = (page / 2).max(1) as i32;
        let next = i32::from(*scroll) + half_pages * step;
        *scroll = next.clamp(0, i32::from(u16::MAX)) as u16;
    }

    pub(super) fn half_changelog(&self) -> i32 {
        (self.changelog_page / 2).max(1) as i32
    }

    pub(super) fn scroll_changelog(&mut self, lines: i32) {
        let next = i32::from(self.changelog_scroll) + lines;
        self.changelog_scroll = next.clamp(0, i32::from(u16::MAX)) as u16;
    }

    pub(super) fn with_focused(&mut self, act: impl Fn(&mut dyn Navigable)) {
        self.reset_scroll();
        match self.focus {
            Stage::Prs => act(&mut self.prs),
            Stage::Queue => act(&mut self.queue),
            Stage::Builds => act(&mut self.builds),
            Stage::Deployed => act(&mut self.deployed),
        }
    }

    pub(super) fn sync_filter(&mut self) {
        let filter = self.filter().map(str::to_string);
        self.prs.filter = filter.clone();
        self.queue.filter = filter.clone();
        self.builds.filter = filter.clone();
        self.deployed.filter = filter;
        self.with_focused(|column| column.select_index_clamped(0));
    }
}
