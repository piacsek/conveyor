use std::io;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variable {
    Text(String),
    Number(u64),
}

impl From<&str> for Variable {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for Variable {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<u64> for Variable {
    fn from(number: u64) -> Self {
        Self::Number(number)
    }
}

impl From<usize> for Variable {
    fn from(number: usize) -> Self {
        Self::Number(number as u64)
    }
}

pub trait Github {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> io::Result<serde_json::Value>;
    fn current_repo(&self) -> io::Result<String>;
}

pub const MISSING: &str =
    "gh not found on PATH; install gh (brew install gh) and run gh auth login";

pub struct CliGh {
    program: PathBuf,
}

impl Default for CliGh {
    fn default() -> Self {
        Self::with_program("gh".into())
    }
}

impl CliGh {
    pub fn with_program(program: PathBuf) -> Self {
        Self { program }
    }

    pub fn graphql_args(query: &str, vars: &[(&str, Variable)]) -> Vec<String> {
        let mut args = vec![
            "api".to_string(),
            "graphql".to_string(),
            "-f".to_string(),
            format!("query={query}"),
        ];
        for (name, value) in vars {
            match value {
                Variable::Text(text) => {
                    args.push("-f".to_string());
                    args.push(format!("{name}={text}"));
                }
                Variable::Number(number) => {
                    args.push("-F".to_string());
                    args.push(format!("{name}={number}"));
                }
            }
        }
        args
    }

    pub fn current_repo_args() -> Vec<String> {
        [
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "--jq",
            ".nameWithOwner",
        ]
        .map(str::to_string)
        .to_vec()
    }

    fn run(&self, args: &[String]) -> io::Result<String> {
        let output = Command::new(&self.program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    io::Error::new(io::ErrorKind::NotFound, MISSING)
                } else {
                    err
                }
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                format!("gh exited with {}", output.status)
            } else {
                stderr
            };
            return Err(io::Error::other(message));
        }
        String::from_utf8(output.stdout)
            .map_err(|err| io::Error::other(format!("gh printed invalid UTF-8: {err}")))
    }
}

impl Github for CliGh {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> io::Result<serde_json::Value> {
        let stdout = self.run(&Self::graphql_args(query, vars))?;
        serde_json::from_str(&stdout)
            .map_err(|err| io::Error::other(format!("gh returned invalid JSON: {err}")))
    }

    fn current_repo(&self) -> io::Result<String> {
        Ok(self.run(&Self::current_repo_args())?.trim().to_string())
    }
}
