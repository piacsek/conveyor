use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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

#[derive(Clone, Default)]
pub struct Log {
    file: Option<Arc<Mutex<File>>>,
}

impl Log {
    pub fn open(path: &Path) -> Log {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut previous = path.as_os_str().to_owned();
        previous.push(".1");
        let _ = fs::rename(path, previous);
        let file = OpenOptions::new().create(true).append(true).open(path).ok();
        Log {
            file: file.map(|file| Arc::new(Mutex::new(file))),
        }
    }

    pub fn line(&self, text: &str) {
        let Some(file) = &self.file else { return };
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        if let Ok(mut file) = file.lock() {
            let _ = file.write_all(format!("{stamp} {text}\n").as_bytes());
        }
    }
}
