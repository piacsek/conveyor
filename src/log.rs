use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn path(
    env_override: Option<PathBuf>,
    xdg_state_home: Option<PathBuf>,
    home: &Path,
    program: &str,
) -> Option<PathBuf> {
    if let Some(file) = env_override.filter(|file| !file.as_os_str().is_empty()) {
        return Some(file);
    }
    let base = xdg_state_home.filter(|dir| dir.is_absolute()).or_else(|| {
        home.is_absolute()
            .then(|| home.join(".local").join("state"))
    })?;
    Some(base.join("conveyor").join(format!("{program}.log")))
}

pub fn program_name(argv0: Option<OsString>) -> String {
    argv0
        .as_deref()
        .and_then(|arg| Path::new(arg).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "conveyor".to_string())
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

    pub fn call(
        &self,
        program: &str,
        args: &[String],
        took: Duration,
        output: &io::Result<Output>,
    ) {
        if self.file.is_none() {
            return;
        }
        let argv: Vec<String> = args.iter().map(|arg| shown(arg)).collect();
        let outcome = match output {
            Ok(output) if output.status.success() => "ok".to_string(),
            Ok(output) => {
                let code = output
                    .status
                    .code()
                    .map_or_else(|| "by signal".to_string(), |code| code.to_string());
                let stderr = String::from_utf8_lossy(&output.stderr);
                let lines: Vec<&str> = stderr
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .collect();
                format!("exit {code}: {}", lines.join(" | "))
            }
            Err(err) => format!("did not start: {err}"),
        };
        self.line(&format!(
            "{program} {} {}ms {outcome}",
            argv.join(" "),
            took.as_millis()
        ));
    }
}

fn shown(arg: &str) -> String {
    match arg.strip_prefix("query=") {
        Some(query) => format!("query=<{} chars>", query.chars().count()),
        None => arg.to_string(),
    }
}
