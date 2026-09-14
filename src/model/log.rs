//! The failed step's log, as `gh run view --log-failed` prints it: one line per log line,
//! `<job>\t<step>\t<timestamp> <text>`.

use crate::text::strip_ansi;

/// The last `keep` lines with the job, step and timestamp prefixes and any ANSI colour
/// removed; trailing blank lines are dropped first.
pub fn tail(text: &str, keep: usize) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(clean).collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let skip = lines.len().saturating_sub(keep);
    lines.drain(..skip);
    lines
}

fn clean(line: &str) -> String {
    let mut fields = line.splitn(3, '\t');
    let text = match (fields.next(), fields.next(), fields.next()) {
        (Some(_job), Some(_step), Some(text)) => text,
        _ => line,
    };
    strip_ansi(without_timestamp(text))
}

/// `2026-09-14T16:09:52.5114315Z cargo test failed` → `cargo test failed`.
fn without_timestamp(text: &str) -> &str {
    let is_stamp = text.len() > 20
        && text.as_bytes()[4] == b'-'
        && text.as_bytes()[7] == b'-'
        && text.as_bytes()[10] == b'T'
        && text[..4].bytes().all(|b| b.is_ascii_digit());
    match (is_stamp, text.find(' ')) {
        (true, Some(space)) => &text[space + 1..],
        _ => text,
    }
}
