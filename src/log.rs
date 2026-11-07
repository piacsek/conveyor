use std::path::{Path, PathBuf};

pub fn path(
    env_override: Option<PathBuf>,
    xdg_state_home: Option<PathBuf>,
    home: &Path,
) -> PathBuf {
    env_override.unwrap_or_else(|| {
        xdg_state_home
            .unwrap_or_else(|| home.join(".local").join("state"))
            .join("conveyor")
            .join("conveyor.log")
    })
}
