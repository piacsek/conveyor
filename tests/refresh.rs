use std::sync::mpsc;
use std::time::{Duration, Instant};

use conveyor::refresh::wait;

#[test]
fn a_burst_of_refresh_signals_wakes_the_fetcher_once() {
    let (tx, rx) = mpsc::channel::<()>();
    tx.send(()).unwrap();
    tx.send(()).unwrap();
    tx.send(()).unwrap();

    assert!(wait(&rx, Duration::from_secs(1)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn the_interval_wakes_the_fetcher_without_a_signal() {
    let (_tx, rx) = mpsc::channel::<()>();
    let started = Instant::now();

    assert!(wait(&rx, Duration::from_millis(10)));
    assert!(started.elapsed() >= Duration::from_millis(10));
}

#[test]
fn a_closed_channel_stops_the_fetcher() {
    let (tx, rx) = mpsc::channel::<()>();
    drop(tx);

    assert!(!wait(&rx, Duration::from_secs(1)));
}
