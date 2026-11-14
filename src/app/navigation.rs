use std::time::SystemTime;

use super::{Action, App, Listing, Mode, Stage};

impl App {
    pub fn filter(&self) -> Option<&str> {
        self.query.as_deref()
    }

    pub fn editing_filter(&self) -> bool {
        matches!(self.mode, Mode::Filter(_))
    }

    pub fn focused_error(&self) -> Option<&str> {
        self.stage(self.focus).error()
    }

    pub fn focused_fetched_at(&self) -> Option<SystemTime> {
        self.stage(self.focus).listing().fetched_at()
    }

    pub fn focused_counts(&self) -> (usize, usize) {
        self.stage(self.focus).listing().counts()
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

    pub(super) fn with_focused(&mut self, act: impl Fn(&mut dyn Listing)) {
        self.reset_scroll();
        act(self.stage_mut(self.focus).listing_mut());
    }

    pub(super) fn sync_filter(&mut self) {
        self.spread_filter();
        self.with_focused(|column| column.select_index_clamped(0));
    }

    fn spread_filter(&mut self) {
        let filter = self.filter().map(str::to_string);
        for stage in Stage::ALL {
            self.stage_mut(stage)
                .listing_mut()
                .set_filter(filter.clone());
        }
    }

    pub(super) fn step_view(&mut self, forward: bool) -> Action {
        let stage = self.stage(self.focus);
        let views = stage.views();
        let step = if forward { 1 } else { views - 1 };
        self.switch_view((stage.active_view() + step) % views)
    }

    pub(super) fn switch_view(&mut self, view: usize) -> Action {
        let stage = self.stage(self.focus);
        if stage.views() < 2 || view >= stage.views() || view == stage.active_view() {
            return Action::Continue;
        }
        self.stage_mut(self.focus).set_active_view(view);
        self.query = self
            .stage(self.focus)
            .listing()
            .filter()
            .map(str::to_string);
        self.spread_filter();
        self.reset_scroll();
        Action::Refresh(self.focus)
    }
}
