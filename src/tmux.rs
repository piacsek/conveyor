use std::io::Read;
use std::process::{Command, Output, Stdio};

use crate::log::Log;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const ASK: &str = "#{client_termname}\t#{version}\t#{client_termfeatures}";
const DEADLINE: Duration = Duration::from_secs(1);

pub const LINKS_NOTICE: &str = "tmux drops the #N links: ? says why";

pub fn links_fix(answer: &str) -> Vec<String> {
    let mut fields = answer.trim_end_matches('\n').split('\t');
    let (Some(term), Some(version)) = (
        fields.next().filter(|term| {
            !term.is_empty()
                && term
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
        }),
        fields.next().filter(|version| printable(version)),
    ) else {
        return Vec::new();
    };
    let features = fields.next().unwrap_or_default();
    if older_than_3_4(version) {
        return vec![format!(
            "tmux {version} drops the #N links: tmux 3.4 or later passes them through"
        )];
    }
    if features.split(',').any(|feature| feature == "hyperlinks") {
        return Vec::new();
    }
    vec![
        "tmux drops the #N links for this terminal. If it shows links, run".to_string(),
        format!("  tmux set -as terminal-features ',{term}:hyperlinks'"),
        "then detach and reattach. To keep it, put this line in tmux.conf:".to_string(),
        format!("  set -as terminal-features ',{term}:hyperlinks'"),
    ]
}

fn printable(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_graphic())
}

fn older_than_3_4(version: &str) -> bool {
    let number = version.strip_prefix("next-").unwrap_or(version);
    let mut parts = number.split('.').map(|part| {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok()
    });
    match (parts.next().flatten(), parts.next().flatten()) {
        (Some(major), Some(minor)) => (major, minor) < (3, 4),
        _ => false,
    }
}

pub fn ask(log: &Log) -> Option<String> {
    std::env::var_os("TMUX")?;
    let args = [
        "display-message".to_string(),
        "-p".to_string(),
        ASK.to_string(),
    ];
    let started = Instant::now();
    let spawned = {
        let _spawning = crate::sources::spawning();
        Command::new("tmux")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    };
    let mut child = match spawned {
        Ok(child) => child,
        Err(err) => {
            log.call("tmux", &args, started.elapsed(), &Err(err));
            return None;
        }
    };
    let mut stdout = child.stdout.take()?;
    let (sender, answer) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        let _ = stdout.read_to_string(&mut text);
        let _ = sender.send(text);
    });
    let deadline = started + DEADLINE;
    let text = answer.recv_timeout(DEADLINE).ok();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    match (status, text) {
        (Some(status), Some(text)) => {
            let output = Output {
                status,
                stdout: text.clone().into_bytes(),
                stderr: Vec::new(),
            };
            log.call("tmux", &args, started.elapsed(), &Ok(output));
            status.success().then_some(text)
        }
        _ => {
            log.timed_out("tmux", &args, started.elapsed(), &[]);
            None
        }
    }
}
