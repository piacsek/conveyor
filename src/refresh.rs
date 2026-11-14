use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use crate::app::{Input, Rows, Stage};

pub fn wait(rx: &Receiver<()>, interval: Duration) -> bool {
    if let Err(RecvTimeoutError::Disconnected) = rx.recv_timeout(interval) {
        return false;
    }
    while rx.try_recv().is_ok() {}
    true
}

pub fn spawn(
    stage: Stage,
    interval: Duration,
    tx: Sender<io::Result<Input>>,
    mut fetch: impl FnMut() -> Result<Rows, String> + Send + 'static,
) -> Sender<()> {
    let (refresh_tx, refresh_rx) = mpsc::channel::<()>();
    thread::spawn(move || {
        loop {
            if tx.send(Ok(Input::Fetching(stage, 0))).is_err() {
                return;
            }
            if tx.send(Ok(Input::Data(stage, 0, fetch()))).is_err() {
                return;
            }
            if !wait(&refresh_rx, interval.max(Duration::from_secs(1))) {
                return;
            }
        }
    });
    refresh_tx
}
