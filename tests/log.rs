use std::path::{Path, PathBuf};

use conveyor::log::path;

#[test]
fn the_log_path_prefers_the_override_then_xdg_state_then_home() {
    let home = Path::new("/home/me");

    assert_eq!(
        path(
            Some(PathBuf::from("/x/c.log")),
            Some(PathBuf::from("/state")),
            home
        ),
        PathBuf::from("/x/c.log")
    );
    assert_eq!(
        path(None, Some(PathBuf::from("/state")), home),
        PathBuf::from("/state/conveyor/conveyor.log")
    );
    assert_eq!(
        path(None, None, home),
        PathBuf::from("/home/me/.local/state/conveyor/conveyor.log")
    );
}
