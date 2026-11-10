use std::path::Path;

const LIMIT: usize = 700;

#[test]
fn no_source_file_is_longer_than_the_limit() {
    assert_eq!(sources_longer_than(LIMIT), "");
}

fn sources_longer_than(limit: usize) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    collect(&root.join("src"), root, limit, &mut found);
    found.sort();
    found.join(" ")
}

fn collect(directory: &Path, root: &Path, limit: usize, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, root, limit, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let lines = std::fs::read_to_string(&path).unwrap().lines().count();
            if lines > limit {
                let name = path.strip_prefix(root).unwrap().display();
                found.push(format!("{name} ({lines})"));
            }
        }
    }
}
