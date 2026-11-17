use std::io;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

use ratatui::crossterm::event::{self, Event};

use crate::app::Input;
use crate::crash;

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
    if let Err(failure) = crash::catch(|| forward_until_stopped(source, clock, tx)) {
        let _ = tx.send(Err(io::Error::other(format!(
            "terminal events stopped: {failure}"
        ))));
    }
}

fn forward_until_stopped(
    source: &mut impl EventSource,
    clock: &impl Clock,
    tx: &Sender<io::Result<Input>>,
) {
    let mut deadline = clock.instant() + TICK;
    loop {
        let now = clock.instant();
        let input = if now >= deadline {
            deadline = now + TICK;
            Input::Tick(clock.system())
        } else {
            match source.poll(deadline - now).and_then(|ready| {
                if ready {
                    source.read().map(Some)
                } else {
                    Ok(None)
                }
            }) {
                Ok(Some(Event::Key(key))) if key.is_press() => Input::Key(key),
                Ok(Some(Event::Resize(..))) => Input::Resize,
                Ok(_) => continue,
                Err(err) => {
                    let _ = tx.send(Err(err));
                    return;
                }
            }
        };
        if tx.send(Ok(input)).is_err() {
            return;
        }
    }
}
