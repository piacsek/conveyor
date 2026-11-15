use std::io;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

use ratatui::crossterm::event::{self, Event};

use crate::app::Input;

const TICK: Duration = Duration::from_millis(250);

pub trait EventSource {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool>;
    fn read(&mut self) -> io::Result<Event>;
}

pub trait Clock {
    fn instant(&self) -> Instant;
    fn system(&self) -> SystemTime;
}

pub struct Crossterm;

impl EventSource for Crossterm {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<Event> {
        event::read()
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn instant(&self) -> Instant {
        Instant::now()
    }

    fn system(&self) -> SystemTime {
        SystemTime::now()
    }
}

pub fn forward(source: &mut impl EventSource, clock: &impl Clock, tx: &Sender<io::Result<Input>>) {
    loop {
        let input = match source.poll(TICK) {
            Ok(true) => match source.read() {
                Ok(Event::Key(key)) if key.is_press() => Input::Key(key),
                Ok(_) => continue,
                Err(err) => {
                    let _ = tx.send(Err(err));
                    return;
                }
            },
            Ok(false) => Input::Tick(clock.system()),
            Err(err) => {
                let _ = tx.send(Err(err));
                return;
            }
        };
        if tx.send(Ok(input)).is_err() {
            return;
        }
    }
}
