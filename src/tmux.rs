use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ASK: &str = "#{client_termname}\t#{version}\t#{client_termfeatures}";
const DEADLINE: Duration = Duration::from_secs(1);

pub fn links_notice(answer: &str) -> Option<String> {
    let mut fields = answer.trim_end_matches('\n').split('\t');
    let term = fields.next().filter(|term| !term.is_empty())?;
    let _version = fields.next()?;
    let features = fields.next().unwrap_or_default();
    if features.split(',').any(|feature| feature == "hyperlinks") {
        return None;
    }
    Some(format!(
        "tmux drops the #N links: add set -as terminal-features ',{term}:hyperlinks' to tmux.conf and reattach"
    ))
}

pub fn ask() -> Option<String> {
    std::env::var_os("TMUX")?;
    let mut child = Command::new("tmux")
        .args(["display-message", "-p", ASK])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(status) if status.success() => break,
            Some(_) => return None,
            None if started.elapsed() >= DEADLINE => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let mut answer = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut answer).ok()?;
    Some(answer)
}
