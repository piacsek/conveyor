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
