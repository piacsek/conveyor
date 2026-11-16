# Phase 46: one generic worker

## Problem

`src/main.rs` has seven worker functions with the same body: the jobs fetcher, the log fetcher,
the rerun worker, the branch updater, the status changer, the checkout worker and the merger.
Each opens a channel, starts a thread, loops on `recv`, calls one function and sends one `Input`
back, returning when the receiver of inputs is gone. Only the request type, the call and the
`Input` variant differ. The loop lives in the binary, so no test covers it, and a change to how
a worker behaves has to be made seven times (#92).

## Goal

One generic worker in the library, covered by tests, and seven calls to it in `src/main.rs`.
Nothing a user sees changes: no snapshot changes, and the fragment is `kind: internal`.

## The shape

A file of its own, `src/worker.rs`, since `refresh::spawn` already exists with another shape:

```rust
pub fn spawn<T, F>(tx: Sender<io::Result<Input>>, work: F) -> Sender<T>
where
    T: Send + 'static,
    F: FnMut(T) -> Input + Send + 'static,
```

The thread ends when the request sender is dropped or when the receiver of inputs is gone.
The terminal event thread and the stage fetchers (`refresh::spawn`) stay as they are: they are
loops of a different shape.

## Tasks

1. `tests/worker.rs` drives `worker::spawn` with a scripted closure: a request comes back as the
   `Input` the closure built; the thread ends when the input receiver is dropped; the thread ends
   when the request sender is dropped. `src/worker.rs` holds the loop.
2. The seven functions in `src/main.rs` become seven `worker::spawn` calls, each with a closure
   that makes its `gh` call and builds its `Input`. Each keeps its own `CliGh` with the call log,
   and the checkout keeps its `HOME` lookup.
3. The fragment `changes/phase-46-generic-worker.md` (`kind: internal`), the `src/worker.rs` line
   in the root `AGENTS.md` layout, and the phase entry in `PLAN.md`.

## Done when

`src/main.rs` holds no worker loop of its own, `tests/worker.rs` covers the generic worker, no
file under `tests/snapshots` changed, and the gates pass.
