use std::sync::mpsc;
use std::time::{Duration, Instant};

use conveyor::app::{Input, Stage};
use conveyor::refresh::{Schedule, Wake, spawn, wait};
use conveyor::sources::failure::Failure;

#[test]
fn a_burst_of_refresh_signals_wakes_the_fetcher_once() {
    let (tx, rx) = mpsc::channel::<usize>();
    tx.send(0).unwrap();
    tx.send(0).unwrap();
    tx.send(0).unwrap();

    assert_eq!(wait(&rx, Duration::from_secs(1)), Wake::Asked(0));
    assert!(rx.try_recv().is_err());
}

#[test]
fn the_last_tab_of_a_burst_is_the_one_fetched() {
    let (tx, rx) = mpsc::channel::<usize>();
    tx.send(1).unwrap();
    tx.send(2).unwrap();

    assert_eq!(wait(&rx, Duration::from_secs(1)), Wake::Asked(2));
}

#[test]
fn the_interval_wakes_the_fetcher_without_a_signal() {
    let (_tx, rx) = mpsc::channel::<usize>();
    let started = Instant::now();

    assert_eq!(wait(&rx, Duration::from_millis(10)), Wake::Due);
    assert!(started.elapsed() >= Duration::from_millis(10));
}

#[test]
fn a_closed_channel_stops_the_fetcher() {
    let (tx, rx) = mpsc::channel::<usize>();
    drop(tx);

    assert_eq!(wait(&rx, Duration::from_secs(1)), Wake::Closed);
}

#[test]
fn refreshes_asked_during_a_fetch_cost_one_more_fetch() {
    let (inputs_tx, inputs) = mpsc::channel();
    let (release_tx, release) = mpsc::channel::<()>();
    let refresh = spawn(
        Stage::Prs,
        Duration::from_secs(60),
        1,
        inputs_tx,
        move |_| {
            release.recv().unwrap();
            Err("done".to_string().into())
        },
    );
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
fn a_sweep_of_two_fetches_both_tabs_at_start_and_again_after_the_interval() {
    let (inputs_tx, inputs) = mpsc::channel();
    let _refresh = spawn(Stage::Prs, Duration::from_secs(1), 2, inputs_tx, |tab| {
        Err(format!("tab {tab}").into())
    });
    let next = || {
        inputs
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
    };

    assert_eq!(next(), Input::Fetching(Stage::Prs, 0));
    assert_eq!(
        next(),
        Input::Data(Stage::Prs, 0, Err("tab 0".to_string().into()))
    );
    assert_eq!(next(), Input::Fetching(Stage::Prs, 1));
    assert_eq!(
        next(),
        Input::Data(Stage::Prs, 1, Err("tab 1".to_string().into()))
    );
    assert_eq!(next(), Input::Fetching(Stage::Prs, 0));
    assert_eq!(
        next(),
        Input::Data(Stage::Prs, 0, Err("tab 0".to_string().into()))
    );
    assert_eq!(next(), Input::Fetching(Stage::Prs, 1));
    assert_eq!(
        next(),
        Input::Data(Stage::Prs, 1, Err("tab 1".to_string().into()))
    );
}

#[test]
fn a_sweep_of_one_fetches_the_first_tab_only() {
    let (inputs_tx, inputs) = mpsc::channel();
    let _refresh = spawn(Stage::Prs, Duration::from_secs(60), 1, inputs_tx, |tab| {
        Err(format!("tab {tab}").into())
    });
    let next = || {
        inputs
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
    };

    assert_eq!(next(), Input::Fetching(Stage::Prs, 0));
    assert_eq!(
        next(),
        Input::Data(Stage::Prs, 0, Err("tab 0".to_string().into()))
    );
    assert!(inputs.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_request_between_sweeps_fetches_its_tab_only() {
    let (inputs_tx, inputs) = mpsc::channel();
    let refresh = spawn(Stage::Prs, Duration::from_secs(60), 2, inputs_tx, |tab| {
        Err(format!("tab {tab}").into())
    });
    let next = || {
        inputs
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
    };
    next();
    next();
    next();
    next();

    refresh.send(1).unwrap();

    assert_eq!(next(), Input::Fetching(Stage::Prs, 1));
    assert_eq!(
        next(),
        Input::Data(Stage::Prs, 1, Err("tab 1".to_string().into()))
    );
    assert!(inputs.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_request_during_a_sweep_is_fetched_before_the_tabs_the_sweep_has_left() {
    let (inputs_tx, inputs) = mpsc::channel();
    let (release_tx, release) = mpsc::channel::<()>();
    let refresh = spawn(
        Stage::Prs,
        Duration::from_secs(60),
        3,
        inputs_tx,
        move |_| {
            release.recv().unwrap();
            Err("done".to_string().into())
        },
    );
    let next = || {
        inputs
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
    };

    assert_eq!(next(), Input::Fetching(Stage::Prs, 0));
    refresh.send(2).unwrap();
    release_tx.send(()).unwrap();
    assert!(matches!(next(), Input::Data(Stage::Prs, 0, _)));
    assert_eq!(next(), Input::Fetching(Stage::Prs, 2));
    release_tx.send(()).unwrap();
    assert!(matches!(next(), Input::Data(Stage::Prs, 2, _)));
    assert_eq!(next(), Input::Fetching(Stage::Prs, 1));
    release_tx.send(()).unwrap();
    assert!(matches!(next(), Input::Data(Stage::Prs, 1, _)));
    assert!(inputs.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_request_between_sweeps_leaves_the_deadline_where_it_was() {
    let start = Instant::now();
    let mut schedule = swept(2, start);

    assert_eq!(schedule.next(Wake::Asked(1), start + secs(1)), Some(1));
    schedule.fetched(start + secs(1));

    assert_eq!(schedule.timeout(start + secs(1)), Some(secs(1)));
}

#[test]
fn the_sweep_after_a_request_starts_with_the_asked_tab() {
    let start = Instant::now();
    let mut schedule = swept(2, start);
    schedule.next(Wake::Asked(1), start + secs(1));
    schedule.fetched(start + secs(1));

    assert_eq!(schedule.next(Wake::Due, start + secs(2)), Some(1));
    schedule.fetched(start + secs(2));
    assert_eq!(schedule.next(Wake::Due, start + secs(2)), Some(0));
}

#[test]
fn a_sweep_of_one_restarts_the_interval_after_a_request() {
    let start = Instant::now();
    let mut schedule = swept(1, start);

    assert_eq!(schedule.next(Wake::Asked(0), start + secs(1)), Some(0));
    schedule.fetched(start + secs(1));

    assert_eq!(schedule.timeout(start + secs(1)), Some(secs(2)));
}

#[test]
fn after_a_request_during_a_sweep_the_next_tab_of_the_sweep_goes_before_any_other_request() {
    let start = Instant::now();
    let mut schedule = Schedule::new(3, secs(60), start);
    schedule.next(Wake::Due, start);
    schedule.fetched(start);

    assert_eq!(schedule.next(Wake::Asked(0), start), Some(0));
    schedule.fetched(start);

    assert_eq!(schedule.timeout(start), None);
    assert_eq!(schedule.next(Wake::Due, start), Some(1));
}

#[test]
fn a_request_after_the_deadline_starts_the_sweep_it_leads() {
    let start = Instant::now();
    let mut schedule = swept(2, start);

    assert_eq!(schedule.next(Wake::Asked(1), start + secs(3)), Some(1));
    schedule.fetched(start + secs(3));

    assert_eq!(schedule.timeout(start + secs(3)), None);
    assert_eq!(schedule.next(Wake::Due, start + secs(3)), Some(0));
}

#[test]
fn a_sweep_of_one_serves_every_request_back_to_back() {
    let start = Instant::now();
    let mut schedule = swept(1, start);
    schedule.next(Wake::Asked(0), start + secs(1));
    schedule.fetched(start + secs(1));

    assert_eq!(schedule.timeout(start + secs(1)), Some(secs(2)));
    assert_eq!(schedule.next(Wake::Asked(0), start + secs(1)), Some(0));
}

#[test]
fn a_sweep_started_by_the_deadline_still_takes_a_request_before_its_next_tab() {
    let start = Instant::now();
    let mut schedule = swept(3, start);

    assert_eq!(schedule.next(Wake::Due, start + secs(2)), Some(0));
    schedule.fetched(start + secs(2));

    assert_eq!(schedule.timeout(start + secs(2)), Some(Duration::ZERO));
}

#[test]
fn a_closed_channel_ends_the_schedule() {
    let start = Instant::now();
    let mut schedule = swept(2, start);

    assert_eq!(schedule.next(Wake::Closed, start), None);
}

#[test]
fn a_refresh_naming_a_tab_fetches_that_tab_and_the_result_carries_it() {
    let (inputs_tx, inputs) = mpsc::channel();
    let refresh = spawn(Stage::Prs, Duration::from_secs(60), 1, inputs_tx, |tab| {
        Err(format!("tab {tab}").into())
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
    assert_eq!(
        next(),
        Input::Data(Stage::Prs, 1, Err("tab 1".to_string().into()))
    );
}

#[test]
fn a_crashed_fetch_comes_back_as_the_columns_error_and_the_next_refresh_fetches() {
    let (inputs_tx, inputs) = mpsc::channel();
    let mut fetches = 0;
    let refresh = spawn(Stage::Queue, secs(60), 1, inputs_tx, move |_| {
        fetches += 1;
        if fetches == 1 {
            panic!("boom");
        }
        Err("fetched again".to_string().into())
    });
    let next = || inputs.recv_timeout(secs(5)).unwrap().unwrap();

    assert_eq!(next(), Input::Fetching(Stage::Queue, 0));
    assert_eq!(
        next(),
        Input::Data(Stage::Queue, 0, Err(Failure::crashed(&"boom")))
    );

    refresh.send(0).unwrap();

    assert_eq!(next(), Input::Fetching(Stage::Queue, 0));
    assert_eq!(
        next(),
        Input::Data(Stage::Queue, 0, Err("fetched again".to_string().into()))
    );
}

#[test]
fn no_profile_aborts_on_a_panic_so_a_crash_stays_catchable() {
    let manifest: toml::Table = include_str!("../Cargo.toml").parse().unwrap();
    let profiles = manifest.get("profile").and_then(toml::Value::as_table);
    let aborting: Vec<&String> = profiles
        .into_iter()
        .flatten()
        .filter(|(_, profile)| profile.get("panic").and_then(toml::Value::as_str) == Some("abort"))
        .map(|(name, _)| name)
        .collect();

    assert!(aborting.is_empty(), "{aborting:?}");
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn swept(tabs: usize, start: Instant) -> Schedule {
    let mut schedule = Schedule::new(tabs, secs(2), start);
    for _ in 0..tabs {
        schedule.next(Wake::Due, start);
        schedule.fetched(start);
    }
    schedule
}
