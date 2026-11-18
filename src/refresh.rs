use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::{Input, Rows, Stage};
use crate::crash;
use crate::sources::failure::Failure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    Watch(usize),
    Once(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Wake {
    Asked {
        watch: Option<usize>,
        once: Vec<usize>,
    },
    Due,
    Closed,
}

pub fn wait(rx: &Receiver<Ask>, timeout: Duration) -> Wake {
    let first = match rx.recv_timeout(timeout) {
        Ok(ask) => ask,
        Err(RecvTimeoutError::Timeout) => return Wake::Due,
        Err(RecvTimeoutError::Disconnected) => return Wake::Closed,
    };
    let mut watch = None;
    let mut once = Vec::new();
    for ask in std::iter::once(first).chain(rx.try_iter()) {
        match ask {
            Ask::Watch(tab) => watch = Some(tab),
            Ask::Once(tab) => once.push(tab),
        }
    }
    Wake::Asked { watch, once }
}

pub struct Schedule {
    tabs: usize,
    interval: Duration,
    first: usize,
    left: Vec<usize>,
    due: Option<Instant>,
    sweeping: bool,
    owed: bool,
    once: Vec<usize>,
    fetching_once: bool,
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
            once: Vec::new(),
            fetching_once: false,
        }
    }

    pub fn timeout(&self, now: Instant) -> Option<Duration> {
        if self.owed || !self.once.is_empty() {
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
        self.fetching_once = false;
        match wake {
            Wake::Closed => None,
            Wake::Asked { watch, once } => {
                for tab in once {
                    if !self.once.contains(&tab) {
                        self.once.push(tab);
                    }
                }
                let Some(asked) = watch else {
                    return self.next(Wake::Due, now);
                };
                self.once.retain(|tab| *tab != asked);
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
                if !self.once.is_empty() {
                    self.fetching_once = true;
                    return Some(self.once.remove(0));
                }
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
        if std::mem::take(&mut self.fetching_once) {
            return;
        }
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
) -> Sender<Ask> {
    let (refresh_tx, refresh_rx) = mpsc::channel::<Ask>();
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
) -> Sender<Ask>
where
    F: FnMut() -> Result<Rows, Failure> + Send + 'static,
{
    let each: Vec<Sender<Ask>> = intervals
        .into_iter()
        .enumerate()
        .map(|(tab, interval)| {
            let (each_tx, each_rx) = mpsc::channel::<Ask>();
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
    let (refresh_tx, refresh_rx) = mpsc::channel::<Ask>();
    let named = thread::Builder::new().name(format!("{stage:?} refresh router"));
    let spawned = named.spawn(move || {
        for ask in refresh_rx {
            let (Ask::Watch(tab) | Ask::Once(tab)) = ask;
            if let Some(fetcher) = each.get(tab) {
                let _ = fetcher.send(Ask::Watch(0));
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
    refresh_rx: Receiver<Ask>,
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
