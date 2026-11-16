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

pub fn spawn(
    stage: Stage,
    interval: Duration,
    tabs: usize,
    tx: Sender<io::Result<Input>>,
    mut fetch: impl FnMut(usize) -> Result<Rows, String> + Send + 'static,
) -> Sender<usize> {
    let (refresh_tx, refresh_rx) = mpsc::channel::<usize>();
    let interval = interval.max(Duration::from_secs(1));
    thread::spawn(move || {
        let mut first = 0;
        let mut left: Vec<usize> = sweep(first, tabs).collect();
        let mut due = Instant::now();
        loop {
            let timeout = if left.is_empty() {
                due.saturating_duration_since(Instant::now())
            } else {
                Duration::ZERO
            };
            let (tab, sweeping) = match wait(&refresh_rx, timeout) {
                Wake::Closed => return,
                Wake::Asked(asked) => {
                    let sweeping = !left.is_empty();
                    first = asked;
                    left.retain(|tab| *tab != asked);
                    (asked, sweeping)
                }
                Wake::Due => {
                    if left.is_empty() {
                        left = sweep(first, tabs).collect();
                    }
                    (left.remove(0), true)
                }
            };
            if tx.send(Ok(Input::Fetching(stage, tab))).is_err() {
                return;
            }
            if tx.send(Ok(Input::Data(stage, tab, fetch(tab)))).is_err() {
                return;
            }
            if left.is_empty() && (sweeping || tabs <= 1) {
                due = Instant::now() + interval;
            }
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
