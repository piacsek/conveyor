use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ASK: &str = "#{client_termname}\t#{version}\t#{client_termfeatures}";
const DEADLINE: Duration = Duration::from_secs(1);

pub const LINKS_NOTICE: &str = "tmux drops the #N links: ? says how to keep them";

pub fn links_fix(answer: &str) -> Vec<String> {
    let mut fields = answer.trim_end_matches('\n').split('\t');
    let (Some(term), Some(version)) = (
        fields.next().filter(|term| printable(term)),
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
        "then detach and reattach; add that line to tmux.conf to keep it.".to_string(),
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
