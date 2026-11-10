use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::config::{MergeMethod, UpdateMethod};
use crate::log::Log;
use crate::model::prs::{BranchUpdate, StatusChange};

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
    fn rest(&self, path: &str) -> io::Result<serde_json::Value>;
    fn current_repo(&self) -> io::Result<String>;
    /// The raw `gh run view --log-failed` text for one job of a run.
    fn log_failed(&self, repo: &str, run_id: u64, job_id: u64) -> io::Result<String>;
    fn rerun_failed(&self, repo: &str, run_id: u64) -> io::Result<()>;
    fn merge(&self, repo: &str, number: u64, method: MergeMethod, head_oid: &str)
    -> io::Result<()>;
    fn update_branch(
        &self,
        repo: &str,
        number: u64,
        method: UpdateMethod,
    ) -> io::Result<BranchUpdate>;
    fn change_status(&self, repo: &str, number: u64, change: &StatusChange) -> io::Result<()>;
    fn checkout(&self, repo: &str, number: u64, dir: Option<&Path>) -> io::Result<()>;
}

const ENABLE_AUTO_MERGE: &str = "mutation($id: ID!, $method: PullRequestMergeMethod!, $head: GitObjectID) { enablePullRequestAutoMerge(input: {pullRequestId: $id, mergeMethod: $method, expectedHeadOid: $head}) { clientMutationId } }";

const DISABLE_AUTO_MERGE: &str = "mutation($id: ID!) { disablePullRequestAutoMerge(input: {pullRequestId: $id}) { clientMutationId } }";

const ENQUEUE: &str = "mutation($id: ID!, $head: GitObjectID) { enqueuePullRequest(input: {pullRequestId: $id, expectedHeadOid: $head}) { clientMutationId } }";

const DEQUEUE: &str =
    "mutation($id: ID!) { dequeuePullRequest(input: {id: $id}) { clientMutationId } }";

pub const MISSING: &str =
    "gh not found on PATH; install gh (brew install gh) and run gh auth login";

pub struct CliGh {
    program: PathBuf,
    log: Log,
}

impl Default for CliGh {
    fn default() -> Self {
        Self::with_program("gh".into())
    }
}

impl CliGh {
    pub fn with_program(program: PathBuf) -> Self {
        Self {
            program,
            log: Log::default(),
        }
    }

    pub fn with_log(self, log: Log) -> Self {
        Self { log, ..self }
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

    pub fn rest_args(path: &str) -> Vec<String> {
        vec!["api".to_string(), path.to_string()]
    }

    pub fn log_failed_args(repo: &str, run_id: u64, job_id: u64) -> Vec<String> {
        [
            "run",
            "view",
            &run_id.to_string(),
            "--job",
            &job_id.to_string(),
            "--log-failed",
            "-R",
            repo,
        ]
        .map(str::to_string)
        .to_vec()
    }

    pub fn rerun_failed_args(repo: &str, run_id: u64) -> Vec<String> {
        ["run", "rerun", &run_id.to_string(), "--failed", "-R", repo]
            .map(str::to_string)
            .to_vec()
    }

    pub fn merge_args(repo: &str, number: u64, method: MergeMethod, head_oid: &str) -> Vec<String> {
        let method = match method {
            MergeMethod::Squash => "--squash",
            MergeMethod::Merge => "--merge",
            MergeMethod::Rebase => "--rebase",
        };
        let mut args = ["pr", "merge", &number.to_string(), "-R", repo, method]
            .map(str::to_string)
            .to_vec();
        if !head_oid.is_empty() {
            args.push("--match-head-commit".to_string());
            args.push(head_oid.to_string());
        }
        args
    }

    pub fn update_branch_args(repo: &str, number: u64, method: UpdateMethod) -> Vec<String> {
        let mut args = ["pr", "update-branch", &number.to_string(), "-R", repo]
            .map(str::to_string)
            .to_vec();
        if method == UpdateMethod::Rebase {
            args.push("--rebase".to_string());
        }
        args
    }

    pub fn status_args(repo: &str, number: u64, change: &StatusChange) -> Vec<String> {
        let pr = |verb: &str| -> Vec<String> {
            ["pr", verb, &number.to_string(), "-R", repo]
                .map(str::to_string)
                .to_vec()
        };
        match change {
            StatusChange::Ready => pr("ready"),
            StatusChange::Draft => [pr("ready"), vec!["--undo".to_string()]].concat(),
            StatusChange::EnableAutoMerge {
                node_id,
                method,
                head_oid,
            } => {
                let method = match method {
                    MergeMethod::Squash => "SQUASH",
                    MergeMethod::Merge => "MERGE",
                    MergeMethod::Rebase => "REBASE",
                };
                let vars = vec![("id", node_id.as_str().into()), ("method", method.into())];
                Self::graphql_args(ENABLE_AUTO_MERGE, &pinned(vars, head_oid))
            }
            StatusChange::DisableAutoMerge { node_id } => {
                Self::graphql_args(DISABLE_AUTO_MERGE, &[("id", node_id.as_str().into())])
            }
            StatusChange::Close => pr("close"),
            StatusChange::Dequeue { node_id } => {
                Self::graphql_args(DEQUEUE, &[("id", node_id.as_str().into())])
            }
            StatusChange::Enqueue { node_id, head_oid } => {
                let vars = vec![("id", node_id.as_str().into())];
                Self::graphql_args(ENQUEUE, &pinned(vars, head_oid))
            }
        }
    }

    pub fn checkout_args(repo: &str, number: u64) -> Vec<String> {
        ["pr", "checkout", &number.to_string(), "-R", repo]
            .map(str::to_string)
            .to_vec()
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
        self.run_in(None, args)
    }

    fn run_in(&self, dir: Option<&Path>, args: &[String]) -> io::Result<String> {
        let started = Instant::now();
        let output = self.output(dir, args).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                io::Error::new(io::ErrorKind::NotFound, MISSING)
            } else {
                err
            }
        });
        self.log.call("gh", args, started.elapsed(), &output);
        let output = output?;
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

    fn output(&self, dir: Option<&Path>, args: &[String]) -> io::Result<std::process::Output> {
        let mut command = Command::new(&self.program);
        if let Some(dir) = dir {
            command.current_dir(dir);
        }
        let child = {
            let _alone = super::spawning();
            command
                .args(args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?
        };
        child.wait_with_output()
    }
}

impl Github for CliGh {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> io::Result<serde_json::Value> {
        let stdout = self.run(&Self::graphql_args(query, vars))?;
        serde_json::from_str(&stdout)
            .map_err(|err| io::Error::other(format!("gh returned invalid JSON: {err}")))
    }

    fn rest(&self, path: &str) -> io::Result<serde_json::Value> {
        let stdout = self.run(&Self::rest_args(path))?;
        serde_json::from_str(&stdout)
            .map_err(|err| io::Error::other(format!("gh returned invalid JSON: {err}")))
    }

    fn current_repo(&self) -> io::Result<String> {
        Ok(self.run(&Self::current_repo_args())?.trim().to_string())
    }

    fn log_failed(&self, repo: &str, run_id: u64, job_id: u64) -> io::Result<String> {
        self.run(&Self::log_failed_args(repo, run_id, job_id))
    }

    fn rerun_failed(&self, repo: &str, run_id: u64) -> io::Result<()> {
        self.run(&Self::rerun_failed_args(repo, run_id)).map(|_| ())
    }

    fn merge(
        &self,
        repo: &str,
        number: u64,
        method: MergeMethod,
        head_oid: &str,
    ) -> io::Result<()> {
        self.run(&Self::merge_args(repo, number, method, head_oid))
            .map(|_| ())
            .map_err(without_warnings)
    }

    fn update_branch(
        &self,
        repo: &str,
        number: u64,
        method: UpdateMethod,
    ) -> io::Result<BranchUpdate> {
        match self.run(&Self::update_branch_args(repo, number, method)) {
            Ok(stdout) if stdout.contains("already up-to-date") => Ok(BranchUpdate::UpToDate),
            Ok(_) => Ok(BranchUpdate::Updated),
            Err(err)
                if err
                    .to_string()
                    .contains("Cannot update PR branch due to conflicts") =>
            {
                Ok(BranchUpdate::Conflicts)
            }
            Err(err) => Err(err),
        }
    }

    fn change_status(&self, repo: &str, number: u64, change: &StatusChange) -> io::Result<()> {
        self.run(&Self::status_args(repo, number, change))
            .map(|_| ())
            .map_err(without_warnings)
    }

    fn checkout(&self, repo: &str, number: u64, dir: Option<&Path>) -> io::Result<()> {
        if let Some(dir) = dir.filter(|dir| !dir.is_dir()) {
            return Err(io::Error::other(format!(
                "{} is not a directory",
                dir.display()
            )));
        }
        let found = self
            .run_in(dir, &Self::current_repo_args())
            .map_err(|err| no_clone(err, repo, dir))?;
        let found = found.trim();
        if !found.eq_ignore_ascii_case(repo) {
            return Err(io::Error::other(match dir {
                Some(dir) => format!("{} is a clone of {found}, not {repo}", dir.display()),
                None => format!(
                    "conveyor runs in a clone of {found}; set `path` under the [[repo]] of {repo}"
                ),
            }));
        }
        self.run_in(dir, &Self::checkout_args(repo, number))
            .map(|_| ())
            .map_err(gits_reason)
    }
}

fn no_clone(err: io::Error, repo: &str, dir: Option<&Path>) -> io::Error {
    if !err.to_string().contains("not a git repository") {
        return err;
    }
    io::Error::other(match dir {
        Some(dir) => format!("{} is not a clone of {repo}", dir.display()),
        None => {
            format!("conveyor does not run in a clone of {repo}; set `path` under its [[repo]]")
        }
    })
}

fn gits_reason(err: io::Error) -> io::Error {
    let message = err.to_string();
    let reason = message
        .lines()
        .find(|line| line.starts_with("fatal: ") || line.starts_with("error: "))
        .or_else(|| message.lines().rfind(|line| !line.trim().is_empty()));
    match reason {
        Some(line) => io::Error::other(line.to_string()),
        None => err,
    }
}

fn pinned<'a>(mut vars: Vec<(&'a str, Variable)>, head_oid: &str) -> Vec<(&'a str, Variable)> {
    if !head_oid.is_empty() {
        vars.push(("head", head_oid.into()));
    }
    vars
}

fn without_warnings(err: io::Error) -> io::Error {
    let message = err.to_string();
    let kept: Vec<&str> = message
        .lines()
        .filter(|line| !line.starts_with("! "))
        .collect();
    if kept.is_empty() {
        return err;
    }
    io::Error::new(err.kind(), kept.join("\n"))
}
