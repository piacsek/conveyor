use std::panic;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use conveyor::crash;
use conveyor::log::Log;

#[test]
fn a_panic_off_the_main_thread_is_logged_and_leaves_the_terminal_alone() {
    let _hook = HOOK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conveyor.log");
    let restored = Arc::new(AtomicBool::new(false));
    install(Log::open(&path), restored.clone());

    let worker = thread::Builder::new()
        .name("merge worker".to_string())
        .spawn(|| panic!("boom"))
        .unwrap();
    let _ = worker.join();
    let _ = panic::take_hook();

    let logged = std::fs::read_to_string(&path).unwrap();
    assert!(
        logged.contains("panic on merge worker: boom at tests/crash.rs:"),
        "{logged}"
    );
    assert!(!restored.load(Ordering::SeqCst));
}

#[test]
fn a_panic_on_the_main_thread_still_restores_the_terminal() {
    let _hook = HOOK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conveyor.log");
    let restored = Arc::new(AtomicBool::new(false));
    install(Log::open(&path), restored.clone());

    let _ = panic::catch_unwind(|| panic!("boom"));
    let _ = panic::take_hook();

    let logged = std::fs::read_to_string(&path).unwrap();
    assert!(logged.contains(": boom at tests/crash.rs:"), "{logged}");
    assert!(restored.load(Ordering::SeqCst));
}

#[test]
fn the_hook_wraps_ratatuis_and_is_in_place_before_the_first_background_thread_starts() {
    let main = std::fs::read_to_string("src/main.rs").unwrap();
    let tui = &main[main.find("fn tui<").unwrap()..];

    let ratatui = tui.find("ratatui::init()").unwrap();
    let hook = tui.find("panic::set_hook(").unwrap();
    let first_thread = tui.find("spawn").unwrap();

    assert!(
        ratatui < hook,
        "ratatui::init() must come before panic::set_hook("
    );
    assert!(
        hook < first_thread,
        "panic::set_hook( must come before the first thread starts"
    );
}

static HOOK: Mutex<()> = Mutex::new(());

fn install(log: Log, restored: Arc<AtomicBool>) {
    panic::set_hook(crash::hook(
        log,
        Box::new(move |_| restored.store(true, Ordering::SeqCst)),
    ));
}
