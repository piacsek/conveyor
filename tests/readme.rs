//! The README is the user-facing contract: four sections, one screenshot, and a key table
//! that mirrors the in-app help so the two can never drift.

use conveyor::ui::help::KEYS;

const README: &str = include_str!("../README.md");

#[test]
fn the_readme_has_exactly_the_four_user_sections_in_order() {
    let headings: Vec<&str> = README
        .lines()
        .filter(|line| line.starts_with("## "))
        .collect();
    assert_eq!(
        headings,
        ["## Installation", "## Usage", "## Development"],
        "one h1, then Installation, Usage, Development"
    );
    assert_eq!(
        README.matches("\n# ").count() + usize::from(README.starts_with("# ")),
        1
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
    assert_eq!(
        description.matches(". ").count() + usize::from(description.ends_with('.')),
        2,
        "two sentences: {description}"
    );
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
