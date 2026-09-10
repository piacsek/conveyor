use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub prs: Prs,
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

impl Config {
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }
}
