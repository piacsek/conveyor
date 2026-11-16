use std::sync::mpsc;
use std::time::Duration;

use conveyor::app::Input;
use conveyor::worker;

#[test]
fn a_request_comes_back_as_the_input_the_work_built() {
    let (inputs_tx, inputs) = mpsc::channel();
    let requests = worker::spawn(inputs_tx, |number: u64| {
        Input::Merged(number, Err(format!("asked for {number}")))
    });

    requests.send(7).unwrap();

    let input = inputs.recv_timeout(WAIT).unwrap().unwrap();
    assert!(matches!(input, Input::Merged(7, Err(ref reason)) if reason == "asked for 7"));
}

#[test]
fn the_worker_ends_once_nobody_reads_its_inputs() {
    let (inputs_tx, inputs) = mpsc::channel();
    let (ended_tx, ended) = mpsc::channel();
    let guard = EndGuard(ended_tx);
    let requests = worker::spawn(inputs_tx, move |number: u64| {
        let _ = &guard;
        Input::Merged(number, Ok(()))
    });
    drop(inputs);

    requests.send(7).unwrap();

    assert!(ended.recv_timeout(WAIT).is_ok());
}

#[test]
fn the_worker_ends_once_nobody_sends_it_requests() {
    let (inputs_tx, _inputs) = mpsc::channel();
    let (ended_tx, ended) = mpsc::channel();
    let guard = EndGuard(ended_tx);
    let requests = worker::spawn(inputs_tx, move |number: u64| {
        let _ = &guard;
        Input::Merged(number, Ok(()))
    });

    drop(requests);

    assert!(ended.recv_timeout(WAIT).is_ok());
}

const WAIT: Duration = Duration::from_secs(5);

struct EndGuard(mpsc::Sender<()>);

impl Drop for EndGuard {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
