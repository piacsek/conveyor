pub const CHANGELOG: &str = include_str!("../CHANGELOG.md");

const UNRELEASED: &str = "0.0.0";

struct Section<'a> {
    heading: &'a str,
    version: &'a str,
    bullets: Vec<String>,
}

fn sections(text: &str) -> Vec<Section<'_>> {
    let mut sections: Vec<Section> = Vec::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let heading = heading.strip_prefix('v').unwrap_or(heading);
            let version = heading.split_whitespace().next().unwrap_or_default();
            sections.push(Section {
                heading: line,
                version,
                bullets: Vec::new(),
            });
        } else if let Some(section) = sections.last_mut() {
            if let Some(bullet) = line.strip_prefix("- ")
                && !bullet.trim().is_empty()
            {
                section.bullets.push(bullet.trim().to_string());
            } else if line.starts_with(char::is_whitespace)
                && !line.trim().is_empty()
                && let Some(bullet) = section.bullets.last_mut()
            {
                bullet.push(' ');
                bullet.push_str(line.trim());
            }
        }
    }
    sections
}

fn dated(heading: &str) -> bool {
    if !heading.starts_with("## v") {
        return false;
    }
    let Some(date) = heading
        .split_once(" (")
        .and_then(|(_, rest)| rest.strip_suffix(')'))
    else {
        return false;
    };
    date.len() == 10
        && date.char_indices().all(|(i, c)| match i {
            4 | 7 => c == '-',
            _ => c.is_ascii_digit(),
        })
        && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
}

fn numbers(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

pub fn check(text: &str, version: &str) -> Result<(), String> {
    let sections = sections(text);
    let newest = sections.first().map_or("none", |section| section.version);
    if version != UNRELEASED && newest != version {
        return Err(format!(
            "CHANGELOG.md has no entry for v{version} at the top (found v{newest})"
        ));
    }
    if let Some(section) = sections.iter().find(|section| !dated(section.heading)) {
        return Err(format!(
            "`{}` must read `## vX.Y.Z (YYYY-MM-DD)`",
            section.heading
        ));
    }
    for pair in sections.windows(2) {
        if numbers(pair[0].version) <= numbers(pair[1].version) {
            return Err(format!(
                "v{} is listed above v{}; newest first, once each",
                pair[0].version, pair[1].version
            ));
        }
    }
    for (i, section) in sections.iter().enumerate() {
        if section.bullets.is_empty() {
            return Err(format!("v{} has no `- ` bullet", section.version));
        }
        for older in &sections[i + 1..] {
            if let Some(bullet) = section.bullets.iter().find(|b| older.bullets.contains(b)) {
                return Err(format!(
                    "v{} repeats a bullet of v{}: {bullet}",
                    section.version, older.version
                ));
            }
        }
    }
    Ok(())
}

pub fn check_released(text: &str, released: &[&str], current: &str) -> Result<(), String> {
    let sections = sections(text);
    match released.iter().find(|version| {
        (current == UNRELEASED || numbers(version) <= numbers(current))
            && !sections.iter().any(|section| section.version == **version)
    }) {
        Some(version) => Err(format!(
            "v{version} was released but has no entry; restore it, or merge origin/main if another branch released it"
        )),
        None => Ok(()),
    }
}
