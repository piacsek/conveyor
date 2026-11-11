use std::path::Path;

const LIMIT: usize = 700;

#[test]
fn no_source_file_is_longer_than_the_limit() {
    assert_eq!(files_longer_than("src", LIMIT), "");
}

#[test]
fn no_test_file_is_longer_than_the_limit() {
    assert_eq!(files_longer_than("tests", LIMIT), "");
}

fn files_longer_than(directory: &str, limit: usize) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    collect(&root.join(directory), root, limit, &mut found);
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
