use std::panic::{self, AssertUnwindSafe};

use crate::sources::failure::Failure;

pub fn catch<R>(work: impl FnOnce() -> R) -> Result<R, Failure> {
    panic::catch_unwind(AssertUnwindSafe(work)).map_err(|payload| Failure::crashed(&*payload))
}
