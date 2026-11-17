use std::cell::Cell;
use std::collections::VecDeque;
use std::io;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use conveyor::app::Input;
use conveyor::events::{Clock, EventSource, forward};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[test]
fn a_key_press_is_forwarded_and_a_release_is_not() {
    let inputs = forwarded(vec![(10, press('p')), (10, release('p'))]);

    assert_eq!(inputs.len(), 2);
    assert!(matches!(&inputs[0], Ok(Input::Key(key)) if key.code == KeyCode::Char('p')));
    assert!(inputs[1].is_err());
}

#[test]
fn a_quiet_terminal_ticks_every_250_ms() {
    let inputs = forwarded(vec![(600, press('j'))]);

    assert_eq!(inputs.len(), 4);
    assert_eq!(tick(&inputs[0]), Some(at(250)));
    assert_eq!(tick(&inputs[1]), Some(at(500)));
    assert!(matches!(&inputs[2], Ok(Input::Key(key)) if key.code == KeyCode::Char('j')));
    assert!(inputs[3].is_err());
}

#[test]
fn ticks_keep_coming_every_250_ms_while_keys_stream() {
    let inputs = forwarded((0..20).map(|_| (50, press('j'))).collect());
    let ticks: Vec<SystemTime> = inputs.iter().filter_map(tick).collect();

    assert_eq!(ticks.len(), 4);
    assert_eq!(ticks[0], at(250));
    assert_eq!(ticks[1], at(500));
    assert_eq!(ticks[2], at(750));
    assert_eq!(ticks[3], at(1000));
}

#[test]
fn ignored_events_do_not_hold_back_the_tick() {
    let inputs = forwarded((0..10).map(|_| (30, release('j'))).collect());

    assert_eq!(inputs.len(), 2);
    assert_eq!(tick(&inputs[0]), Some(at(250)));
    assert!(inputs[1].is_err());
}

#[test]
fn a_resize_is_forwarded_so_run_draws_at_the_new_size() {
    let inputs = forwarded(vec![(10, Event::Resize(100, 30))]);

    assert_eq!(inputs.len(), 2);
    assert!(matches!(&inputs[0], Ok(Input::Resize)));
    assert!(inputs[1].is_err());
}

#[test]
fn a_failed_read_is_forwarded_and_ends_the_loop() {
    let inputs = forwarded(vec![]);

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].as_ref().unwrap_err().to_string(), "script over");
}

#[test]
fn the_loop_ends_when_nobody_listens() {
    let elapsed = Rc::new(Cell::new(Duration::ZERO));
    let mut source = Scripted {
        elapsed: elapsed.clone(),
        script: VecDeque::from(vec![(10, press('p')), (10, press('p'))]),
    };
    let (tx, rx) = mpsc::channel();
    drop(rx);

    forward(&mut source, &Fake { elapsed }, &tx);

    assert_eq!(source.script.len(), 1);
}

#[test]
fn a_crash_in_the_events_loop_is_forwarded_as_an_error_that_ends_the_app() {
    let (tx, rx) = mpsc::channel();

    forward(
        &mut Crashing,
        &Fake {
            elapsed: Rc::default(),
        },
        &tx,
    );
    drop(tx);
    let inputs: Vec<io::Result<Input>> = rx.into_iter().collect();

    assert_eq!(inputs.len(), 1);
    assert_eq!(
        inputs[0].as_ref().unwrap_err().to_string(),
        "terminal events stopped: crashed: boom"
    );
}

fn forwarded(script: Vec<(u64, Event)>) -> Vec<io::Result<Input>> {
    let elapsed = Rc::new(Cell::new(Duration::ZERO));
    let mut source = Scripted {
        elapsed: elapsed.clone(),
        script: script.into(),
    };
    let (tx, rx) = mpsc::channel();
    forward(&mut source, &Fake { elapsed }, &tx);
    drop(tx);
    rx.into_iter().collect()
}

fn tick(input: &io::Result<Input>) -> Option<SystemTime> {
    match input {
        Ok(Input::Tick(now)) => Some(*now),
        _ => None,
    }
}

fn at(millis: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(millis)
}

fn press(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn release(c: char) -> Event {
    Event::Key(KeyEvent::new_with_kind(
        KeyCode::Char(c),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ))
}

struct Scripted {
    elapsed: Rc<Cell<Duration>>,
    script: VecDeque<(u64, Event)>,
}

impl EventSource for Scripted {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        let Some((delay, _)) = self.script.front_mut() else {
            return Err(io::Error::other("script over"));
        };
        let wait = Duration::from_millis(*delay);
        if wait <= timeout {
            self.elapsed.set(self.elapsed.get() + wait);
            *delay = 0;
            Ok(true)
        } else {
            self.elapsed.set(self.elapsed.get() + timeout);
            *delay -= timeout.as_millis() as u64;
            Ok(false)
        }
    }

    fn read(&mut self) -> io::Result<Event> {
        self.script
            .pop_front()
            .map(|(_, event)| event)
            .ok_or_else(|| io::Error::other("script over"))
    }
}

struct Crashing;

impl EventSource for Crashing {
    fn poll(&mut self, _: Duration) -> io::Result<bool> {
        panic!("boom");
    }

    fn read(&mut self) -> io::Result<Event> {
        panic!("boom");
    }
}

struct Fake {
    elapsed: Rc<Cell<Duration>>,
}

impl Clock for Fake {
    fn instant(&self) -> Instant {
        START.with(|start| *start) + self.elapsed.get()
    }

    fn system(&self) -> SystemTime {
        UNIX_EPOCH + self.elapsed.get()
    }
}

thread_local! {
    static START: Instant = Instant::now();
}
