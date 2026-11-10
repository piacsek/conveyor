pub mod fetch;
pub mod github;
pub mod kube;

use std::sync::{Mutex, MutexGuard};

static SPAWNING: Mutex<()> = Mutex::new(());

pub(crate) fn spawning() -> MutexGuard<'static, ()> {
    SPAWNING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
