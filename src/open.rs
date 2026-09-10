use std::io;
use std::io::Write;
use std::process::{Command, Stdio};

pub trait Opener {
    fn open(&self, url: &str) -> io::Result<()>;
    fn copy(&self, text: &str) -> io::Result<()>;
}

#[derive(Default)]
pub struct SystemOpener;

impl SystemOpener {
    pub fn open_args(url: &str) -> Vec<String> {
        let program = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        vec![program.to_string(), url.to_string()]
    }

    pub fn copy_args() -> Vec<String> {
        if cfg!(target_os = "macos") {
            vec!["pbcopy".to_string()]
        } else {
            ["xclip", "-selection", "clipboard"]
                .map(str::to_string)
                .to_vec()
        }
    }
}

fn check(name: &str, status: std::process::ExitStatus) -> io::Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("{name} exited with {status}")))
    }
}

fn not_found(name: &str, err: io::Error) -> io::Error {
    if err.kind() == io::ErrorKind::NotFound {
        io::Error::new(io::ErrorKind::NotFound, format!("{name} not found on PATH"))
    } else {
        err
    }
}

impl Opener for SystemOpener {
    fn open(&self, url: &str) -> io::Result<()> {
        let args = Self::open_args(url);
        let status = Command::new(&args[0])
            .args(&args[1..])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|err| not_found(&args[0], err))?;
        check(&args[0], status)
    }

    fn copy(&self, text: &str) -> io::Result<()> {
        let args = Self::copy_args();
        let mut child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| not_found(&args[0], err))?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(text.as_bytes())?;
        }
        check(&args[0], child.wait()?)
    }
}
