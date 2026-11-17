use std::fs;
use std::path::{Path, PathBuf};

const FIRST_PHASE_WITH_HURDLES: u32 = 40;
const FIRST_PHASE_WITH_METRICS: u32 = 53;
const FIRST_SWEEP: u32 = 60;
const SWEEP_EVERY: u32 = 5;
const TAGS: [&str; 7] = [
    "review-loop",
    "ci",
    "tooling",
    "test-seam",
    "process",
    "api-shape",
    "design",
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn plan_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(repo().join("plans"))
        .expect("plans/")
        .map(|entry| entry.expect("plans/ entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    files
}

fn phase_number(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let rest = name.strip_prefix("phase-")?;
    let (number, _) = rest.split_once('-')?;
    number.parse().ok()
}

fn phases() -> Vec<(u32, PathBuf)> {
    plan_files()
        .into_iter()
        .filter_map(|path| phase_number(&path).map(|n| (n, path)))
        .collect()
}

fn section<'a>(text: &'a str, heading: &str) -> Option<Vec<&'a str>> {
    let mut lines = text.lines();
    let dated = format!("{heading} (");
    lines.find(|line| line.trim_end() == heading || line.starts_with(&dated))?;
    Some(lines.take_while(|line| !line.starts_with("## ")).collect())
}

fn bullets(lines: &[&str]) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in lines {
        if let Some(item) = line.strip_prefix("- ") {
            items.push(item.to_string());
        } else if let (Some(last), true) = (items.last_mut(), line.starts_with("  ")) {
            last.push(' ');
            last.push_str(line.trim());
        }
    }
    items
}

fn hurdle_is_well_formed(item: &str) -> bool {
    let Some(rest) = item.strip_prefix('`') else {
        return false;
    };
    let Some((tag, rest)) = rest.split_once('`') else {
        return false;
    };
    let Some((what, outcome)) = rest.rsplit_once(" → ") else {
        return false;
    };
    let outcome_known = matches!(outcome, "code" | "backlog" | "known limit" | "none")
        || outcome
            .strip_prefix("rule in `")
            .and_then(|path| path.strip_suffix('`'))
            .is_some_and(|path| repo().join(path).is_file());
    TAGS.contains(&tag) && !what.trim().is_empty() && outcome_known
}

#[test]
fn every_phase_plan_has_an_outcome_section() {
    let missing: Vec<String> = phases()
        .into_iter()
        .filter(|(_, path)| section(&read(path), "## Outcome").is_none())
        .map(|(_, path)| path.display().to_string())
        .collect();
    assert!(missing.is_empty(), "no `## Outcome` in: {missing:?}");
}

#[test]
fn every_recent_phase_plan_has_well_formed_hurdles() {
    let mut problems = Vec::new();
    for (n, path) in phases() {
        if n < FIRST_PHASE_WITH_HURDLES {
            continue;
        }
        let text = read(&path);
        let Some(lines) = section(&text, "## Hurdles") else {
            problems.push(format!("{}: no `## Hurdles`", path.display()));
            continue;
        };
        let items = bullets(&lines);
        if items.len() == 1 && items[0] == "none recorded" {
            continue;
        }
        for item in items.iter().filter(|item| !hurdle_is_well_formed(item)) {
            problems.push(format!("{}: {item}", path.display()));
        }
    }
    assert!(
        problems.is_empty(),
        "each hurdle reads \"- `<tag>` <what happened> → <code|backlog|known limit|none|rule in `<path>`>\", \
         tags {TAGS:?}, or the section is \"- none recorded\":\n{}",
        problems.join("\n")
    );
}

fn opens_with_a_sweep(n: u32) -> bool {
    n >= FIRST_SWEEP && n.is_multiple_of(SWEEP_EVERY)
}

#[test]
fn every_sweep_phase_holds_its_sweep_in_its_outcome() {
    let missing: Vec<String> = phases()
        .into_iter()
        .filter(|(n, _)| opens_with_a_sweep(*n))
        .filter(|(_, path)| {
            section(&read(path), "## Outcome")
                .is_none_or(|lines| !lines.iter().any(|line| line.trim_end() == "### Sweep"))
        })
        .map(|(_, path)| path.display().to_string())
        .collect();
    assert!(
        missing.is_empty(),
        "a phase whose number is a multiple of {SWEEP_EVERY} opens its `## Outcome` with `### Sweep`: {missing:?}"
    );
}

#[test]
fn sweeps_are_due_from_phase_60_every_fifth_phase() {
    assert!(!opens_with_a_sweep(55));
    assert!(!opens_with_a_sweep(59));
    assert!(opens_with_a_sweep(60));
    assert!(!opens_with_a_sweep(61));
    assert!(opens_with_a_sweep(65));
}

#[test]
fn every_recent_phase_has_a_metrics_row() {
    let metrics = read(&repo().join("plans/metrics.md"));
    let missing: Vec<u32> = phases()
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| *n >= FIRST_PHASE_WITH_METRICS)
        .filter(|n| {
            !metrics
                .lines()
                .any(|line| line.starts_with(&format!("| {n} |")))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "plans/metrics.md has no row for phases {missing:?}"
    );
}

#[test]
fn the_plan_index_links_every_plan_file() {
    let index = read(&repo().join("PLAN.md"));
    let missing: Vec<String> = plan_files()
        .into_iter()
        .filter_map(|path| path.file_name().and_then(|n| n.to_str()).map(String::from))
        .filter(|name| !index.contains(&format!("(plans/{name})")))
        .collect();
    assert!(missing.is_empty(), "PLAN.md links none of: {missing:?}");
}

#[test]
fn a_hurdle_names_its_tag_what_happened_and_where_it_went() {
    assert!(hurdle_is_well_formed("`ci` the job flaked → code"));
    assert!(hurdle_is_well_formed(
        "`process` a rule was missing → rule in `AGENTS.md`"
    ));
    assert!(!hurdle_is_well_formed("`ci` the job flaked"));
    assert!(!hurdle_is_well_formed("`flaky` the job flaked → code"));
    assert!(!hurdle_is_well_formed(
        "`ci` the job flaked → rule in `NOPE.md`"
    ));
    assert!(!hurdle_is_well_formed("the job flaked → code"));
}
