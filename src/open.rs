use std::io;
use std::io::Write;
use std::process::{Command, Stdio};

pub trait Opener {
    fn open(&self, url: &str) -> io::Result<()>;
    fn copy(&self, text: &str) -> io::Result<()>;
}

#[derive(Default)]
pub struct SystemOpener;

/// Accept a URL for the browser only when it is a web URL. Check URLs come from whatever
/// GitHub App or CI posted the check, not from GitHub itself, and `open`/`xdg-open` would
/// otherwise launch `file://` paths, app bundles or any custom URL scheme handler.
pub fn web_url(url: &str) -> io::Result<&str> {
    let scheme_ok = |scheme: &str| {
        url.get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
            && url.len() > scheme.len()
    };
    if scheme_ok("https://") || scheme_ok("http://") {
        Ok(url)
    } else {
        Err(io::Error::other(format!(
            "refusing to open `{url}`: not an http(s) URL"
        )))
    }
}

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
        let args = Self::open_args(web_url(url)?);
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
