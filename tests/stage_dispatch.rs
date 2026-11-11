use std::path::Path;

const DISPATCH: &str = "src/app/stages/mod.rs";

#[test]
fn only_the_dispatch_file_matches_on_the_stage() {
    assert_eq!(stage_arms_outside(DISPATCH), "");
}

fn stage_arms_outside(allowed: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    collect(&root.join("src/app"), root, allowed, &mut found);
    found.sort();
    found.join(" ")
}

fn collect(directory: &Path, root: &Path, allowed: &str, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        let name = path.strip_prefix(root).unwrap().display().to_string();
        if path.is_dir() {
            collect(&path, root, allowed, found);
        } else if name.ends_with(".rs") && name != allowed {
            let source = std::fs::read_to_string(&path).unwrap();
            for (index, line) in source.lines().enumerate() {
                if is_stage_arm(line) {
                    found.push(format!("{name}:{}", index + 1));
                }
            }
        }
    }
}

fn is_stage_arm(line: &str) -> bool {
    let line = line.trim_start();
    let arm = (line.starts_with("Stage::") || line.starts_with("(Stage::")) && line.contains("=>");
    let matched = line.contains("matches!(") && line.contains("Stage::");
    arm || matched
}
