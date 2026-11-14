use std::sync::mpsc;
use std::time::{Duration, Instant};

use conveyor::app::{Input, Stage};
use conveyor::refresh::{spawn, wait};

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

#[test]
fn refreshes_asked_during_a_fetch_cost_one_more_fetch() {
    let (inputs_tx, inputs) = mpsc::channel();
    let (release_tx, release) = mpsc::channel::<()>();
    let refresh = spawn(Stage::Prs, Duration::from_secs(60), inputs_tx, move || {
        release.recv().unwrap();
        Err("done".to_string())
    });
    let next = || {
        inputs
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
    };

    assert!(matches!(next(), Input::Fetching(Stage::Prs, 0)));
    refresh.send(()).unwrap();
    refresh.send(()).unwrap();
    refresh.send(()).unwrap();
    release_tx.send(()).unwrap();
    assert!(matches!(next(), Input::Data(Stage::Prs, 0, _)));
    assert!(matches!(next(), Input::Fetching(Stage::Prs, 0)));
    release_tx.send(()).unwrap();
    assert!(matches!(next(), Input::Data(Stage::Prs, 0, _)));
    assert!(inputs.recv_timeout(Duration::from_millis(300)).is_err());
}
