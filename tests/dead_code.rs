use std::path::Path;

const SEAMS: &[&str] = &["check_released", "with_deadline", "with_git"];

const SUPPORT: &str = "tests/support/mod.rs";

const KINDS: &[&str] = &["fn", "struct", "enum", "const", "static", "trait", "type"];

#[test]
fn every_public_item_in_src_is_referenced_in_src() {
    assert_eq!(unreferenced_public_items(), "");
}

#[test]
fn every_listed_seam_is_declared_in_src_and_used_by_a_test() {
    assert_eq!(stale_seams(), "");
}

#[test]
fn every_support_helper_is_used_by_a_test() {
    assert_eq!(unused_support_helpers(), "");
}

#[test]
fn dead_code_is_allowed_only_in_the_support_module() {
    assert_eq!(dead_code_allowances(), "");
}

struct Source {
    path: String,
    lines: Vec<String>,
}

struct Item {
    path: String,
    line: usize,
    kind: String,
    name: String,
}

impl Item {
    fn label(&self) -> String {
        format!("{}:{} {}", self.path, self.line, self.name)
    }
}

fn unreferenced_public_items() -> String {
    let src = sources("src");
    let mut found: Vec<String> = declarations(&src)
        .iter()
        .filter(|item| !SEAMS.contains(&item.name.as_str()))
        .filter(|item| !referenced(&src, &item.name, Some(item), true))
        .map(Item::label)
        .collect();
    found.sort();
    found.join(" ")
}

fn stale_seams() -> String {
    let declared = declarations(&sources("src"));
    let tests = sources("tests");
    let mut found = Vec::new();
    for seam in SEAMS {
        if !declared.iter().any(|item| item.name == *seam) {
            found.push(format!("{seam} (not declared in src)"));
        }
        if !referenced(&tests, seam, None, false) {
            found.push(format!("{seam} (not used under tests)"));
        }
    }
    found.join(" ")
}

fn unused_support_helpers() -> String {
    let tests = sources("tests");
    let mut found: Vec<String> = declarations(&tests)
        .iter()
        .filter(|item| item.path == SUPPORT && item.kind == "fn")
        .filter(|item| !referenced(&tests, &item.name, Some(item), false))
        .map(Item::label)
        .collect();
    found.sort();
    found.join(" ")
}

fn dead_code_allowances() -> String {
    let mut found = Vec::new();
    let all = sources("src").into_iter().chain(sources("tests"));
    for source in all.filter(|source| source.path != SUPPORT) {
        for (index, line) in source.lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') && contains_word(trimmed, "dead_code") {
                found.push(format!("{}:{}", source.path, index + 1));
            }
        }
    }
    found.sort();
    found.join(" ")
}

fn sources(directory: &str) -> Vec<Source> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    collect(&root.join(directory), root, &mut found);
    found
}

fn collect(directory: &Path, root: &Path, found: &mut Vec<Source>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, root, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            found.push(Source {
                path: path.strip_prefix(root).unwrap().display().to_string(),
                lines: text.lines().map(str::to_string).collect(),
            });
        }
    }
}

fn declarations(sources: &[Source]) -> Vec<Item> {
    let mut found = Vec::new();
    for source in sources {
        for (index, line) in source.lines.iter().enumerate() {
            if let Some((kind, name)) = declared(line) {
                found.push(Item {
                    path: source.path.clone(),
                    line: index + 1,
                    kind,
                    name,
                });
            }
        }
    }
    found
}

fn declared(line: &str) -> Option<(String, String)> {
    let mut words = after_visibility(line)?.split_whitespace().peekable();
    let mut kind = words.next()?;
    if matches!(kind, "const" | "async" | "unsafe") && words.peek() == Some(&"fn") {
        kind = words.next()?;
    }
    if !KINDS.contains(&kind) {
        return None;
    }
    let name: String = words.next()?.chars().take_while(|c| is_ident(*c)).collect();
    (!name.is_empty()).then(|| (kind.to_string(), name))
}

fn after_visibility(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("pub")?;
    match rest.strip_prefix('(') {
        Some(scoped) => Some(&scoped[scoped.find(')')? + 1..]),
        None => rest.starts_with(char::is_whitespace).then_some(rest),
    }
}

fn referenced(sources: &[Source], name: &str, own: Option<&Item>, skip_pub_use: bool) -> bool {
    sources.iter().any(|source| {
        let mut in_pub_use = false;
        source.lines.iter().enumerate().any(|(index, line)| {
            if skip_pub_use && (in_pub_use || is_pub_use(line)) {
                in_pub_use = !line.contains(';');
                return false;
            }
            let is_own = own.is_some_and(|item| item.path == source.path && item.line == index + 1);
            !is_own && contains_word(line, name)
        })
    })
}

fn is_pub_use(line: &str) -> bool {
    after_visibility(line).is_some_and(|rest| rest.trim_start().starts_with("use "))
}

fn contains_word(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let after = line[start + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
