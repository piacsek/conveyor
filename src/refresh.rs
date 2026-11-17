use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::{Input, Rows, Stage};
use crate::crash;
use crate::sources::failure::Failure;

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
    due: Option<Instant>,
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
            due: Some(now),
            sweeping: false,
            owed: false,
        }
    }

    pub fn timeout(&self, now: Instant) -> Option<Duration> {
        if self.owed {
            None
        } else if self.left.is_empty() {
            Some(
                self.due
                    .map_or(Duration::MAX, |due| due.saturating_duration_since(now)),
            )
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
                let tab = if self.left.is_empty() && self.due.is_some_and(|due| now >= due) {
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
            self.due = now.checked_add(self.interval);
        }
    }
}

pub fn spawn(
    stage: Stage,
    interval: Duration,
    tabs: usize,
    tx: Sender<io::Result<Input>>,
    fetch: impl FnMut(usize) -> Result<Rows, Failure> + Send + 'static,
) -> Sender<usize> {
    let (refresh_tx, refresh_rx) = mpsc::channel::<usize>();
    let schedule = Schedule::new(tabs, interval, Instant::now());
    start(
        format!("{stage:?} fetcher"),
        stage,
        schedule,
        refresh_rx,
        tx,
        |tab| tab,
        fetch,
    );
    refresh_tx
}

pub fn spawn_each<F>(
    stage: Stage,
    intervals: Vec<Duration>,
    tx: Sender<io::Result<Input>>,
    make_fetch: impl Fn(usize) -> F,
) -> Sender<usize>
where
    F: FnMut() -> Result<Rows, Failure> + Send + 'static,
{
    let each: Vec<Sender<usize>> = intervals
        .into_iter()
        .enumerate()
        .map(|(tab, interval)| {
            let (each_tx, each_rx) = mpsc::channel::<usize>();
            let mut fetch = make_fetch(tab);
            let schedule = Schedule::new(1, interval, Instant::now());
            let name = format!("{stage:?} fetcher {tab}");
            start(
                name,
                stage,
                schedule,
                each_rx,
                tx.clone(),
                move |_| tab,
                move |_| fetch(),
            );
            each_tx
        })
        .collect();
    let (refresh_tx, refresh_rx) = mpsc::channel::<usize>();
    let named = thread::Builder::new().name(format!("{stage:?} refresh router"));
    let spawned = named.spawn(move || {
        for tab in refresh_rx {
            if let Some(fetcher) = each.get(tab) {
                let _ = fetcher.send(0);
            }
        }
    });
    spawned.expect("the refresh router thread starts");
    refresh_tx
}

fn start(
    name: String,
    stage: Stage,
    mut schedule: Schedule,
    refresh_rx: Receiver<usize>,
    tx: Sender<io::Result<Input>>,
    shown_as: impl Fn(usize) -> usize + Send + 'static,
    mut fetch: impl FnMut(usize) -> Result<Rows, Failure> + Send + 'static,
) {
    let named = thread::Builder::new().name(name);
    let spawned = named.spawn(move || {
        loop {
            let wake = match schedule.timeout(Instant::now()) {
                Some(timeout) => wait(&refresh_rx, timeout),
                None => Wake::Due,
            };
            let Some(tab) = schedule.next(wake, Instant::now()) else {
                return;
            };
            let shown = shown_as(tab);
            if tx.send(Ok(Input::Fetching(stage, shown))).is_err() {
                return;
            }
            let data = crash::catch(|| fetch(tab)).and_then(|data| data);
            if tx.send(Ok(Input::Data(stage, shown, data))).is_err() {
                return;
            }
            schedule.fetched(Instant::now());
        }
    });
    spawned.expect("the fetcher thread starts");
}

fn sweep(first: usize, tabs: usize) -> impl Iterator<Item = usize> {
    std::iter::once(first).chain(
        (0..tabs)
            .filter(move |tab| *tab != first)
            .take(tabs.saturating_sub(1)),
    )
}
