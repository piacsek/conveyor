use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Action, App, Mode, Stage};

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.notice = None;
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if control && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        match self.mode {
            Mode::Help => {
                self.mode = Mode::Normal;
                Action::Continue
            }
            Mode::Changelog => {
                match key.code {
                    KeyCode::Char('d') if control => self.scroll_changelog(self.half_changelog()),
                    KeyCode::Char('u') if control => self.scroll_changelog(-self.half_changelog()),
                    KeyCode::Char('j') | KeyCode::Down => self.scroll_changelog(1),
                    KeyCode::Char('k') | KeyCode::Up => self.scroll_changelog(-1),
                    _ => self.mode = Mode::Normal,
                }
                Action::Continue
            }
            Mode::Confirm(_) => self.confirm(key),
            Mode::Manage(_) => self.manage(key),
            _ if control && key.code == KeyCode::Char('d') => {
                self.scroll_details(1);
                Action::Continue
            }
            _ if control && key.code == KeyCode::Char('u') => {
                self.scroll_details(-1);
                Action::Continue
            }
            Mode::Filter(_) => self.handle_filter_key(key),
            Mode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char(c) => self.edit_filter(|q| q.push(c)),
            KeyCode::Backspace => self.edit_filter(|q| {
                q.pop();
            }),
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                if self.query.as_deref() == Some("") {
                    self.query = None;
                }
                self.sync_filter();
                Action::Continue
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.clear_filter();
                Action::Continue
            }
            _ => self.handle_normal_key(key),
        }
    }

    fn edit_filter(&mut self, edit: impl FnOnce(&mut String)) -> Action {
        if let Mode::Filter(query) = &mut self.mode {
            edit(query);
            self.query = Some(query.clone());
        }
        self.sync_filter();
        Action::Continue
    }

    fn clear_filter(&mut self) {
        self.query = None;
        self.sync_filter();
    }

    /// `Esc` closes the topmost thing, one per press: the details pane, then zoom, then a
    /// live filter. With nothing open it does nothing; it never quits.
    fn escape(&mut self) {
        if self.details {
            self.details = false;
            self.reset_scroll();
        } else if self.zoom {
            self.zoom = false;
        } else if self.query.is_some() {
            self.clear_filter();
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Action {
        let pending_g = std::mem::take(&mut self.pending_g);
        match key.code {
            KeyCode::Char('q') => return Action::Quit,
            KeyCode::Esc => self.escape(),
            KeyCode::Char(digit @ '1'..='4') => {
                let index = digit as usize - '1' as usize;
                self.refocus(|_| Stage::ALL[index]);
            }
            KeyCode::Char('/') => {
                let current = self.query.clone().unwrap_or_default();
                self.query = Some(current.clone());
                self.mode = Mode::Filter(current);
                self.sync_filter();
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Char('c') => {
                self.changelog_scroll = 0;
                self.mode = Mode::Changelog;
            }
            KeyCode::Char('p') => match self.pull_target() {
                Ok((_, url)) => return self.open(url),
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('b') => match self.build_url() {
                Ok(url) => return self.open(url),
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('y') => match self.pull_target() {
                Ok((number, url)) => {
                    self.notice = Some(format!("copied #{number}"));
                    return Action::Copy(url);
                }
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('Y') => match self.build_url() {
                Ok(url) => {
                    self.notice = Some("copied the build URL".to_string());
                    return Action::Copy(url);
                }
                Err(message) => self.notice = Some(message),
            },
            KeyCode::Char('r') => return Action::Refresh(self.focus),
            KeyCode::Char('R') => return Action::RefreshAll,
            KeyCode::Char('d') => {
                self.details = !self.details;
                self.reset_scroll();
            }
            KeyCode::Char('L') => return self.toggle_log(),
            KeyCode::Char('S') => self.ask_to_manage(),
            KeyCode::Char('z') => self.zoom = !self.zoom,
            KeyCode::Char('l') if self.focus != Stage::Deployed => self.refocus(Stage::next),
            KeyCode::Char('h') if self.focus != Stage::Prs => self.refocus(Stage::previous),
            KeyCode::Tab => self.refocus(Stage::next),
            KeyCode::BackTab => self.refocus(Stage::previous),
            KeyCode::Char('j') | KeyCode::Down => self.with_focused(|c| c.next()),
            KeyCode::Char('k') | KeyCode::Up => self.with_focused(|c| c.previous()),
            KeyCode::Char('G') => self.with_focused(|c| c.last()),
            KeyCode::Char('g') if pending_g => self.with_focused(|c| c.first()),
            KeyCode::Char('g') => self.pending_g = true,
            _ => {}
        }
        Action::Continue
    }
}
