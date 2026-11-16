use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use crate::app::{Input, Rows, Stage};

pub fn wait(rx: &Receiver<usize>, interval: Duration, tab: usize) -> Option<usize> {
    let mut tab = match rx.recv_timeout(interval) {
        Ok(asked) => asked,
        Err(RecvTimeoutError::Timeout) => tab,
        Err(RecvTimeoutError::Disconnected) => return None,
    };
    while let Ok(asked) = rx.try_recv() {
        tab = asked;
    }
    Some(tab)
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
        let mut first = 0;
        loop {
            for tab in sweep(first, tabs) {
                if tx.send(Ok(Input::Fetching(stage, tab))).is_err() {
                    return;
                }
                if tx.send(Ok(Input::Data(stage, tab, fetch(tab)))).is_err() {
                    return;
                }
            }
            match wait(&refresh_rx, interval.max(Duration::from_secs(1)), first) {
                Some(asked) => first = asked,
                None => return,
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
