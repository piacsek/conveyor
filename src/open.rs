use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub trait Opener {
    fn open(&self, url: &str) -> io::Result<()>;
    fn copy(&self, text: &str) -> io::Result<()>;
}

pub struct SystemOpener {
    open_program: PathBuf,
}

impl Default for SystemOpener {
    fn default() -> Self {
        Self::with_open_program(Self::default_open_program().into())
    }
}

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
    /// The browser launcher to spawn instead of the platform's `open`/`xdg-open`; tests
    /// point it at a program that does not exist so nothing is ever launched.
    pub fn with_open_program(open_program: PathBuf) -> Self {
        Self { open_program }
    }

    fn default_open_program() -> &'static str {
        if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        }
    }

    pub fn open_args(url: &str) -> Vec<String> {
        vec![Self::default_open_program().to_string(), url.to_string()]
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
        let name = self.open_program.display().to_string();
        let status = Command::new(&self.open_program)
            .arg(web_url(url)?)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|err| not_found(&name, err))?;
        check(&name, status)
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
