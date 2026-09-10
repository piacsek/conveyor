use std::time::{Duration, SystemTime};

pub fn truncate(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut cut: String = text.chars().take(width - 1).collect();
    cut.push('…');
    cut
}

pub fn wrap(text: &str, width: usize, lines: usize) -> Vec<String> {
    if width == 0 || lines == 0 {
        return Vec::new();
    }
    let mut wrapped: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.chars().count() > width {
            if !current.is_empty() {
                wrapped.push(std::mem::take(&mut current));
            }
            wrapped.push(word.chars().take(width).collect());
            word = word.chars().skip(width).collect();
        }
        if current.is_empty() {
            current = word;
        } else if current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(&word);
        } else {
            wrapped.push(std::mem::take(&mut current));
            current = word;
        }
    }
    if !current.is_empty() {
        wrapped.push(current);
    }
    if wrapped.len() > lines {
        wrapped.truncate(lines);
        if let Some(last) = wrapped.last_mut() {
            *last = truncate(&format!("{last} …"), width);
        }
    }
    wrapped
}

pub fn pad_right(text: &str, width: usize) -> String {
    let count = text.chars().count();
    let mut padded = truncate(text, width);
    if count < width {
        padded.extend(std::iter::repeat_n(' ', width - count));
    }
    padded
}

pub fn age(since: SystemTime, now: SystemTime) -> String {
    duration(now.duration_since(since).unwrap_or(Duration::ZERO))
}

pub fn duration(value: Duration) -> String {
    let secs = value.as_secs();
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

pub fn clock(at: SystemTime, utc_offset_secs: i32) -> String {
    let secs = at
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + i64::from(utc_offset_secs);
    let day = secs.rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", day / 3600, day % 3600 / 60, day % 60)
}

pub fn refreshed(at: SystemTime, now: SystemTime, utc_offset_secs: i32) -> String {
    let elapsed = now.duration_since(at).unwrap_or(Duration::ZERO);
    if elapsed < Duration::from_secs(3) {
        "refreshed just now".to_string()
    } else {
        format!("refreshed at {}", clock(at, utc_offset_secs))
    }
}
