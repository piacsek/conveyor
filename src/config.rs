use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

mod values;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub prs: Prs,
    pub ui: Ui,
    pub repo: Vec<Repo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, into = "PrsToml")]
pub struct Prs {
    pub query: String,
    pub limit: usize,
    pub refresh_secs: u64,
    pub merge_method: MergeMethod,
    pub update_method: UpdateMethod,
    pub background_tabs: bool,
    pub tab: Vec<PrsTab>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrsTab {
    pub name: String,
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Serialize)]
struct PrsToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    limit: usize,
    refresh_secs: u64,
    merge_method: MergeMethod,
    update_method: UpdateMethod,
    background_tabs: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tab: Vec<PrsTab>,
}

impl Prs {
    pub fn tabs(&self) -> Vec<PrsTab> {
        if self.tab.is_empty() {
            return vec![PrsTab {
                name: String::new(),
                query: self.query.clone(),
                limit: Some(self.limit),
            }];
        }
        self.tab
            .iter()
            .map(|tab| PrsTab {
                limit: Some(tab.limit.unwrap_or(self.limit)),
                ..tab.clone()
            })
            .collect()
    }
}

impl From<Prs> for PrsToml {
    fn from(prs: Prs) -> Self {
        Self {
            query: prs.tab.is_empty().then_some(prs.query),
            limit: prs.limit,
            refresh_secs: prs.refresh_secs,
            merge_method: prs.merge_method,
            update_method: prs.update_method,
            background_tabs: prs.background_tabs,
            tab: prs.tab,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeMethod {
    #[default]
    Squash,
    Merge,
    Rebase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateMethod {
    #[default]
    Merge,
    Rebase,
}

impl Default for Prs {
    fn default() -> Self {
        Self {
            query: "is:pr is:open archived:false (author:@me OR assignee:@me)".to_string(),
            limit: 20,
            refresh_secs: 60,
            merge_method: MergeMethod::default(),
            update_method: UpdateMethod::default(),
            background_tabs: true,
            tab: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    pub min_column_width: u16,
    pub details_percent: u16,
    pub icons: Icons,
    pub keys: Keys,
    #[serde(skip_serializing_if = "Titles::is_unset")]
    pub titles: Titles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Icons {
    #[default]
    Plain,
    Nerd,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Titles {
    pub prs: Option<String>,
    pub queue: Option<String>,
    pub builds: Option<String>,
    pub deployed: Option<String>,
}

impl Titles {
    fn is_unset(&self) -> bool {
        *self == Titles::default()
    }

    fn check(&self) -> Result<(), String> {
        let titles = [
            ("prs", &self.prs),
            ("queue", &self.queue),
            ("builds", &self.builds),
            ("deployed", &self.deployed),
        ];
        for (key, title) in titles {
            let Some(title) = title else { continue };
            if title.trim().is_empty() {
                return Err(format!(
                    "[ui.titles] {key} is blank; remove it or name the column"
                ));
            }
            if title.trim() != title {
                return Err(format!("[ui.titles] {key} has spaces around it"));
            }
            if title.chars().any(char::is_control) {
                return Err(format!(
                    "[ui.titles] {key} holds a control character; use visible text"
                ));
            }
        }
        Ok(())
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            min_column_width: 36,
            details_percent: 40,
            icons: Icons::default(),
            keys: Keys::default(),
            titles: Titles::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Keys {
    pub next_tab: Key,
    pub previous_tab: Key,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            next_tab: Key::Tab,
            previous_tab: Key::BackTab,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Key {
    Char(char),
    Tab,
    BackTab,
    Ctrl(char),
}

const IN_USE: &[(&str, &str)] = &[
    ("q", "quit"),
    ("jk", "move"),
    ("gG", "first / last row"),
    ("hl", "focus column"),
    ("123456789", "nth tab"),
    ("p", "open pull request"),
    ("b", "open build"),
    ("yY", "copy pull request / build URL"),
    ("d", "details"),
    ("L", "log of the failed step"),
    ("S", "manage"),
    ("z", "zoom the column"),
    ("/", "filter"),
    ("rR", "refresh"),
    ("?", "help"),
    ("c", "changelog"),
];

const CTRL_IN_USE: &[(&str, &str)] = &[("c", "quit"), ("du", "scroll the details")];

const CTRL_SENT_AS: &[(&str, &str)] = &[
    ("i", "Tab"),
    ("mj", "Enter"),
    ("h", "Backspace"),
    ("[", "Esc"),
];

impl Key {
    pub fn parse(name: &str) -> Result<Key, String> {
        let single = |text: &str| {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Some(c),
                _ => None,
            }
        };
        let (key, in_use) = match name {
            "tab" => return Ok(Key::Tab),
            "shift-tab" => return Ok(Key::BackTab),
            _ => match (single(name), name.strip_prefix("ctrl-").and_then(single)) {
                (Some(c), _) if c.is_whitespace() => {
                    return Err(format!("\"{name}\" is blank; use a visible character"));
                }
                (Some(c), _) => (Key::Char(c), meaning(IN_USE, c)),
                (None, Some(c)) => {
                    if let Some(sent) = meaning(CTRL_SENT_AS, c) {
                        return Err(format!(
                            "\"{name}\" reaches conveyor as {sent}: terminals send the same code for both"
                        ));
                    }
                    if !c.is_ascii_lowercase() {
                        return Err(format!(
                            "\"{name}\" is not a control chord terminals send; use ctrl-a to ctrl-z"
                        ));
                    }
                    (Key::Ctrl(c), meaning(CTRL_IN_USE, c))
                }
                (None, None) => {
                    return Err(format!(
                        "unknown key \"{name}\"; use one character, \"tab\", \"shift-tab\" or \"ctrl-<char>\""
                    ));
                }
            },
        };
        match in_use {
            Some(what) => Err(format!("\"{name}\" is already the key for {what}")),
            None => Ok(key),
        }
    }
}

fn meaning(table: &[(&str, &'static str)], c: char) -> Option<&'static str> {
    table
        .iter()
        .find(|(keys, _)| keys.contains(c))
        .map(|(_, what)| *what)
}

impl TryFrom<String> for Key {
    type Error = String;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Key::parse(&name)
    }
}

impl From<Key> for String {
    fn from(key: Key) -> Self {
        key.to_string()
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(c) => write!(f, "{c}"),
            Key::Tab => write!(f, "tab"),
            Key::BackTab => write!(f, "shift-tab"),
            Key::Ctrl(c) => write!(f, "ctrl-{c}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Repo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub main_workflow: String,
    pub branch: Option<String>,
    pub builds: usize,
    pub refresh_secs: u64,
    pub deploy: Vec<Deploy>,
}

impl Default for Repo {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: None,
            main_workflow: "CI/CD".to_string(),
            branch: None,
            builds: 10,
            refresh_secs: 30,
            deploy: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Deploy {
    pub system: String,
    pub refresh_secs: u64,
    pub env: Vec<DeployEnv>,
}

impl Default for Deploy {
    fn default() -> Self {
        Self {
            system: String::new(),
            refresh_secs: 120,
            env: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeployEnv {
    pub name: String,
    pub fetcher: Fetcher,
    pub context: String,
    pub namespace: String,
    pub deployment: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fetcher {
    #[default]
    Kubectl,
}

impl Config {
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct ConfigError {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for ConfigError {}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(err) => {
            return Err(ConfigError {
                path: path.to_path_buf(),
                message: err.to_string(),
            });
        }
    };
    let error = |message: String| ConfigError {
        path: path.to_path_buf(),
        message,
    };
    let config: Config =
        toml::from_str(&text).map_err(|err| error(err.to_string().trim().to_string()))?;
    check_tabs(&text, &config.prs).map_err(error)?;
    config.ui.titles.check().map_err(error)?;
    check_branches(&config.repo).map_err(error)?;
    values::check(&config).map_err(error)?;
    if config.ui.keys.next_tab == config.ui.keys.previous_tab {
        return Err(error(format!(
            "next_tab and previous_tab are both \"{}\"",
            config.ui.keys.next_tab
        )));
    }
    Ok(config)
}

fn check_branches(repos: &[Repo]) -> Result<(), String> {
    for repo in repos {
        let Some(branch) = repo.branch.as_deref() else {
            continue;
        };
        let trimmed = branch.trim();
        if trimmed.is_empty() {
            return Err(format!(
                "the branch of {} is blank; remove it or name the branch",
                repo.name
            ));
        }
        if trimmed != branch {
            return Err(format!(
                "the branch of {} has spaces around it; write it as \"{trimmed}\"",
                repo.name
            ));
        }
    }
    Ok(())
}

fn check_tabs(text: &str, prs: &Prs) -> Result<(), String> {
    if prs.tab.is_empty() {
        return Ok(());
    }
    let table: toml::Table = toml::from_str(text).map_err(|err| err.to_string())?;
    if table.get("prs").and_then(|prs| prs.get("query")).is_some() {
        return Err(
            "[prs].query and [[prs.tab]] are both set; move the query into a tab".to_string(),
        );
    }
    if prs.tab.iter().any(|tab| tab.name.trim().is_empty()) {
        return Err("a [[prs.tab]] has a blank name".to_string());
    }
    for (i, tab) in prs.tab.iter().enumerate() {
        if prs.tab[..i].iter().any(|other| other.name == tab.name) {
            return Err(format!("two [[prs.tab]] are named \"{}\"", tab.name));
        }
    }
    Ok(())
}

pub fn path(
    env_override: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    home: &Path,
) -> PathBuf {
    env_override.unwrap_or_else(|| {
        xdg_config_home
            .unwrap_or_else(|| home.join(".config"))
            .join("conveyor")
            .join("config.toml")
    })
}

pub fn expand_home(path: &Path, home: Option<&Path>) -> PathBuf {
    match (path.strip_prefix("~"), home) {
        (Ok(rest), Some(home)) => home.join(rest),
        _ => path.to_path_buf(),
    }
}
