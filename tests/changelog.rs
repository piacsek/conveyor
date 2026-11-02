const CHANGELOG: &str = include_str!("../CHANGELOG.md");

fn versions() -> Vec<&'static str> {
    CHANGELOG
        .lines()
        .filter_map(|line| line.strip_prefix("## v"))
        .map(|rest| rest.split_whitespace().next().unwrap_or_default())
        .collect()
}

#[test]
fn the_newest_section_is_the_crate_version() {
    assert_eq!(versions().first().copied(), Some(env!("CARGO_PKG_VERSION")));
}
