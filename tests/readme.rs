//! The README is the user-facing contract: four sections, one screenshot, and a key table
//! that mirrors the in-app help so the two can never drift.

use conveyor::ui::help::KEYS;

const README: &str = include_str!("../README.md");

/// The README lines outside fenced code blocks: a `# comment` inside a ```sh block is not a
/// heading.
fn prose() -> Vec<&'static str> {
    let mut fenced = false;
    README
        .lines()
        .filter(|line| {
            if line.starts_with("```") {
                fenced = !fenced;
                return false;
            }
            !fenced
        })
        .collect()
}

/// Sentence ends: `.`, `!` or `?` followed by whitespace or the end, so `e.g. x` and
/// `v0.12.1` do not count.
fn sentences(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    chars
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|next| next.is_whitespace())
        })
        .count()
}

#[test]
fn the_readme_has_exactly_the_four_user_sections_in_order() {
    let prose = prose();
    let sections: Vec<&str> = prose
        .iter()
        .copied()
        .filter(|line| line.starts_with("## "))
        .collect();
    assert_eq!(
        sections,
        ["## Installation", "## Usage", "## Development"],
        "Installation, Usage, Development and nothing else"
    );
    assert_eq!(
        prose.iter().filter(|line| line.starts_with("# ")).count(),
        1,
        "one h1"
    );
    assert!(
        !prose.iter().any(|line| line.starts_with("### ")),
        "no subsections"
    );
}

#[test]
fn the_readme_embeds_one_screenshot_right_below_the_description() {
    let images: Vec<&str> = README
        .lines()
        .filter(|line| line.starts_with("!["))
        .collect();
    assert_eq!(images.len(), 1, "one screenshot: {images:?}");
    assert!(images[0].contains("(docs/details.png)"), "{}", images[0]);
    let before_image = README.split("![").next().unwrap();
    assert!(
        !before_image.contains("\n## "),
        "the screenshot comes before the first section"
    );
    let description = before_image
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(sentences(&description), 2, "two sentences: {description}");
}

#[test]
fn the_readme_key_table_mirrors_the_in_app_help() {
    let rows: Vec<&str> = README
        .lines()
        .filter(|line| line.starts_with("| `"))
        .collect();
    let expected: Vec<String> = KEYS
        .iter()
        .map(|(key, what)| format!("| `{key}` | {what} |"))
        .collect();
    assert_eq!(
        rows, expected,
        "one row per KEYS entry, same order, same words"
    );
}
