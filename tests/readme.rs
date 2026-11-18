use conveyor::ui::help::KEYS;

const README: &str = include_str!("../README.md");

/// The README lines outside fenced code blocks: a `# comment` inside a ```sh block is not a
/// heading.
fn prose(readme: &str) -> Vec<&str> {
    let mut fenced = false;
    readme
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
fn the_readme_has_exactly_the_five_user_sections_in_order() {
    let prose = prose(README);
    let sections: Vec<&str> = prose
        .iter()
        .copied()
        .filter(|line| line.starts_with("## "))
        .collect();
    assert_eq!(
        sections,
        [
            "## Installation",
            "## Dependencies",
            "## Usage",
            "## FAQ",
            "## Development"
        ],
        "Installation, Dependencies, Usage, FAQ, Development and nothing else"
    );
    assert!(prose[0].starts_with("<img "), "a banner opens the README");
    assert!(
        !prose.iter().any(|line| line.starts_with("# ")),
        "the banner stands in for the h1"
    );
    assert!(
        !prose.iter().any(|line| line.starts_with("### ")),
        "no subsections"
    );
}

#[test]
fn the_readme_embeds_the_screenshot_and_one_recording_right_below_the_description() {
    assert_eq!(images(README), 2, "the screenshot and the recording");
    let before_image = README.split("![").next().unwrap();
    assert!(
        !before_image.contains("\n## "),
        "the screenshot comes before the first section"
    );
    let description = before_image
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("<img "))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(sentences(&description), 2, "two sentences: {description}");
}

#[test]
fn the_recording_sits_right_below_the_screenshot() {
    let lines: Vec<&str> = README.lines().collect();
    let screenshot = lines
        .iter()
        .position(|line| line.contains("(docs/details.png)"))
        .expect("the screenshot");
    assert!(
        standalone(lines[screenshot], "docs/details.png"),
        "{}",
        lines[screenshot]
    );
    assert_eq!(lines[screenshot + 1], "");
    assert!(
        standalone(lines[screenshot + 2], "docs/demo.gif"),
        "{}",
        lines[screenshot + 2]
    );
}

#[test]
fn an_html_image_counts() {
    assert_eq!(
        images(&format!("{README}\n<img src=\"docs/plain.png\">\n")),
        3
    );
}

#[test]
fn an_indented_image_counts() {
    assert_eq!(images(&format!("{README}\n  ![x](docs/plain.png)\n")), 3);
}

#[test]
fn a_linked_image_counts() {
    assert_eq!(
        images(&format!(
            "{README}\n[![x](docs/plain.png)](https://example.com)\n"
        )),
        3
    );
}

#[test]
fn an_image_after_the_screenshot_on_its_line_counts() {
    let readme = README.replace(
        "(docs/details.png)",
        "(docs/details.png) ![x](docs/plain.png)",
    );
    assert_eq!(images(&readme), 3);
}

#[test]
fn an_image_after_the_screenshot_on_its_line_leaves_it_not_standalone() {
    assert!(!standalone(
        "![a](docs/details.png) ![b](docs/plain.png)",
        "docs/details.png"
    ));
}

#[test]
fn an_image_before_the_screenshot_on_its_line_leaves_it_not_standalone() {
    assert!(!standalone(
        "![b](docs/plain.png) ![a](docs/details.png)",
        "docs/details.png"
    ));
}

#[test]
fn the_recording_comes_from_the_committed_tape() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let tape = std::fs::read_to_string(root.join("scripts/demo.tape")).unwrap_or_default();
    assert!(
        tape.lines().any(|line| line == "Output docs/demo.gif"),
        "{tape}"
    );
    assert!(root.join("docs/demo.gif").is_file());
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

#[test]
fn the_readme_default_config_loads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, toml_block()).unwrap();

    let loaded = conveyor::config::load(&path);

    assert!(loaded.is_ok(), "{:?}", loaded.err());
}

#[test]
fn the_readme_nix_install_works_without_the_experimental_features_enabled() {
    assert!(README.contains(
        "\nnix --extra-experimental-features 'nix-command flakes' profile install github:piacsek/conveyor/stable\n"
    ));
}

fn toml_block() -> String {
    README
        .split("```toml\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("a ```toml block")
        .to_string()
}

fn images(readme: &str) -> usize {
    prose(readme)
        .iter()
        .skip(1)
        .map(|line| image_marks(line))
        .sum()
}

fn image_marks(line: &str) -> usize {
    line.matches("![").count() + line.matches("<img").count()
}

fn standalone(line: &str, path: &str) -> bool {
    line.starts_with("![") && line.ends_with(&format!("]({path})")) && image_marks(line) == 1
}
