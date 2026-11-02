use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

pub fn wait(rx: &Receiver<()>, interval: Duration) -> bool {
    if let Err(RecvTimeoutError::Disconnected) = rx.recv_timeout(interval) {
        return false;
    }
    while rx.try_recv().is_ok() {}
    true
}
