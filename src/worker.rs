use std::io;
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::app::Input;

pub fn spawn<T, F>(tx: Sender<io::Result<Input>>, mut work: F) -> Sender<T>
where
    T: Send + 'static,
    F: FnMut(T) -> Input + Send + 'static,
{
    let (requests_tx, requests) = mpsc::channel::<T>();
    thread::spawn(move || {
        while let Ok(request) = requests.recv() {
            if tx.send(Ok(work(request))).is_err() {
                return;
            }
        }
    });
    requests_tx
}
