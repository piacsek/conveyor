use std::time::Duration;

use super::{Action, App};

pub const OPEN_DEBOUNCE: Duration = Duration::from_secs(1);

impl App {
    pub(super) fn pull_target(&self) -> Result<(u64, String), String> {
        self.stage(self.focus).pull_target()
    }

    pub(super) fn build_url(&self) -> Result<String, String> {
        self.stage(self.focus).build_url(&self.context())
    }

    pub(super) fn open(&mut self, url: String) -> Action {
        if let Err(err) = crate::open::web_url(&url) {
            self.notice = Some(err.to_string());
            return Action::Continue;
        }
        if self.opened_recently(&url) {
            return Action::Continue;
        }
        self.last_open = Some((url.clone(), self.now));
        Action::Open(url)
    }

    fn opened_recently(&self, url: &str) -> bool {
        self.last_open.as_ref().is_some_and(|(last, at)| {
            last == url
                && self
                    .now
                    .duration_since(*at)
                    .is_ok_and(|elapsed| elapsed < OPEN_DEBOUNCE)
        })
    }
}
