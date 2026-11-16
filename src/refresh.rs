use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::{Input, Rows, Stage};

#[derive(Debug, PartialEq, Eq)]
pub enum Wake {
    Asked(usize),
    Due,
    Closed,
}

pub fn wait(rx: &Receiver<usize>, timeout: Duration) -> Wake {
    let mut tab = match rx.recv_timeout(timeout) {
        Ok(asked) => asked,
        Err(RecvTimeoutError::Timeout) => return Wake::Due,
        Err(RecvTimeoutError::Disconnected) => return Wake::Closed,
    };
    while let Ok(asked) = rx.try_recv() {
        tab = asked;
    }
    Wake::Asked(tab)
}

pub struct Schedule {
    tabs: usize,
    interval: Duration,
    first: usize,
    left: Vec<usize>,
    due: Instant,
    sweeping: bool,
    owed: bool,
}

impl Schedule {
    pub fn new(tabs: usize, interval: Duration, now: Instant) -> Self {
        Self {
            tabs,
            interval: interval.max(Duration::from_secs(1)),
            first: 0,
            left: sweep(0, tabs).collect(),
            due: now,
            sweeping: false,
            owed: false,
        }
    }

    pub fn timeout(&self, now: Instant) -> Option<Duration> {
        if self.owed {
            None
        } else if self.left.is_empty() {
            Some(self.due.saturating_duration_since(now))
        } else {
            Some(Duration::ZERO)
        }
    }

    pub fn next(&mut self, wake: Wake, now: Instant) -> Option<usize> {
        self.owed = false;
        match wake {
            Wake::Closed => None,
            Wake::Asked(asked) => {
                self.first = asked;
                let tab = if self.left.is_empty() && now >= self.due {
                    self.start_sweep()
                } else {
                    self.sweeping = !self.left.is_empty();
                    self.left.retain(|tab| *tab != asked);
                    asked
                };
                self.owed = !self.left.is_empty();
                Some(tab)
            }
            Wake::Due => {
                if self.left.is_empty() {
                    return Some(self.start_sweep());
                }
                self.sweeping = true;
                Some(self.left.remove(0))
            }
        }
    }

    fn start_sweep(&mut self) -> usize {
        self.left = sweep(self.first, self.tabs).collect();
        self.sweeping = true;
        self.left.remove(0)
    }

    pub fn fetched(&mut self, now: Instant) {
        if self.left.is_empty() && (self.sweeping || self.tabs <= 1) {
            self.due = now + self.interval;
        }
    }
}

pub fn spawn(
    stage: Stage,
    interval: Duration,
    tabs: usize,
    tx: Sender<io::Result<Input>>,
    mut fetch: impl FnMut(usize) -> Result<Rows, String> + Send + 'static,
) -> Sender<usize> {
    let (refresh_tx, refresh_rx) = mpsc::channel::<usize>();
    thread::spawn(move || {
        let mut schedule = Schedule::new(tabs, interval, Instant::now());
        loop {
            let wake = match schedule.timeout(Instant::now()) {
                Some(timeout) => wait(&refresh_rx, timeout),
                None => Wake::Due,
            };
            let Some(tab) = schedule.next(wake, Instant::now()) else {
                return;
            };
            if tx.send(Ok(Input::Fetching(stage, tab))).is_err() {
                return;
            }
            if tx.send(Ok(Input::Data(stage, tab, fetch(tab)))).is_err() {
                return;
            }
            schedule.fetched(Instant::now());
        }
    });
    refresh_tx
}

fn sweep(first: usize, tabs: usize) -> impl Iterator<Item = usize> {
    std::iter::once(first).chain(
        (0..tabs)
            .filter(move |tab| *tab != first)
            .take(tabs.saturating_sub(1)),
    )
}
