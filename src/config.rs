use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub prs: Prs,
    pub ui: Ui,
    pub repo: Vec<Repo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Prs {
    pub query: String,
    pub limit: usize,
    pub refresh_secs: u64,
}

impl Default for Prs {
    fn default() -> Self {
        Self {
            query: "is:pr is:open author:@me archived:false".to_string(),
            limit: 20,
            refresh_secs: 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    pub min_column_width: u16,
    pub details_percent: u16,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            min_column_width: 36,
            details_percent: 40,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Repo {
    pub name: String,
    pub main_workflow: String,
    pub builds: usize,
    pub refresh_secs: u64,
}

impl Default for Repo {
    fn default() -> Self {
        Self {
            name: String::new(),
            main_workflow: "CI/CD".to_string(),
            builds: 10,
            refresh_secs: 30,
        }
    }
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
    toml::from_str(&text).map_err(|err| ConfigError {
        path: path.to_path_buf(),
        message: err.to_string().trim().to_string(),
    })
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
