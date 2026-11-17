use std::any::Any;
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::thread;

use crate::log::Log;
use crate::sources::failure::Failure;

pub type Hook = Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync>;

pub fn catch<R>(work: impl FnOnce() -> R) -> Result<R, Failure> {
    panic::catch_unwind(AssertUnwindSafe(work)).map_err(|payload| Failure::crashed(&*payload))
}

pub fn hook(log: Log, main: Hook) -> Hook {
    let installer = thread::current().id();
    Box::new(move |info| {
        let current = thread::current();
        let at = info
            .location()
            .map(|location| format!(" at {}:{}", location.file(), location.line()))
            .unwrap_or_default();
        log.line(&format!(
            "panic on {}: {}{at}",
            current.name().unwrap_or("<unnamed>"),
            said(info.payload())
        ));
        if current.id() == installer {
            main(info);
        }
    })
}

pub fn said(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a panic with no message")
}
