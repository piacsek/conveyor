use std::sync::mpsc;
use std::time::{Duration, Instant};

use conveyor::app::{Input, Stage};
use conveyor::refresh::{spawn, wait};

#[test]
fn a_burst_of_refresh_signals_wakes_the_fetcher_once() {
    let (tx, rx) = mpsc::channel::<usize>();
    tx.send(0).unwrap();
    tx.send(0).unwrap();
    tx.send(0).unwrap();

    assert_eq!(wait(&rx, Duration::from_secs(1), 0), Some(0));
    assert!(rx.try_recv().is_err());
}

#[test]
fn the_last_tab_of_a_burst_is_the_one_fetched() {
    let (tx, rx) = mpsc::channel::<usize>();
    tx.send(1).unwrap();
    tx.send(2).unwrap();

    assert_eq!(wait(&rx, Duration::from_secs(1), 0), Some(2));
}

#[test]
fn the_interval_wakes_the_fetcher_without_a_signal() {
    let (_tx, rx) = mpsc::channel::<usize>();
    let started = Instant::now();

    assert_eq!(wait(&rx, Duration::from_millis(10), 1), Some(1));
    assert!(started.elapsed() >= Duration::from_millis(10));
}

#[test]
fn a_closed_channel_stops_the_fetcher() {
    let (tx, rx) = mpsc::channel::<usize>();
    drop(tx);

    assert_eq!(wait(&rx, Duration::from_secs(1), 0), None);
}

#[test]
fn refreshes_asked_during_a_fetch_cost_one_more_fetch() {
    let (inputs_tx, inputs) = mpsc::channel();
    let (release_tx, release) = mpsc::channel::<()>();
    let refresh = spawn(Stage::Prs, Duration::from_secs(60), inputs_tx, move |_| {
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
    refresh.send(0).unwrap();
    refresh.send(0).unwrap();
    refresh.send(0).unwrap();
    release_tx.send(()).unwrap();
    assert!(matches!(next(), Input::Data(Stage::Prs, 0, _)));
    assert!(matches!(next(), Input::Fetching(Stage::Prs, 0)));
    release_tx.send(()).unwrap();
    assert!(matches!(next(), Input::Data(Stage::Prs, 0, _)));
    assert!(inputs.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_refresh_naming_a_tab_fetches_that_tab_and_the_result_carries_it() {
    let (inputs_tx, inputs) = mpsc::channel();
    let refresh = spawn(Stage::Prs, Duration::from_secs(60), inputs_tx, |tab| {
        Err(format!("tab {tab}"))
    });
    let next = || {
        inputs
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
    };
    next();
    next();

    refresh.send(1).unwrap();

    assert_eq!(next(), Input::Fetching(Stage::Prs, 1));
    assert_eq!(next(), Input::Data(Stage::Prs, 1, Err("tab 1".to_string())));
}
