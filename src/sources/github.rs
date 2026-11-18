use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::{MergeMethod, UpdateMethod};
use crate::log::Log;
use crate::model::prs::{BranchUpdate, StatusChange};
use crate::sources::failure::Failure;
use crate::sources::runner::{self, Ran};

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
    fn graphql(&self, query: &str, vars: &[(&str, Variable)])
    -> Result<serde_json::Value, Failure>;
    fn rest(&self, path: &str) -> Result<serde_json::Value, Failure>;
    fn current_repo(&self) -> Result<String, Failure>;
    /// The raw `gh run view --log-failed` text for one job of a run.
    fn log_failed(&self, repo: &str, run_id: u64, job_id: u64) -> Result<String, Failure>;
    fn rerun_failed(&self, repo: &str, run_id: u64) -> Result<(), Failure>;
    fn merge(
        &self,
        repo: &str,
        number: u64,
        method: MergeMethod,
        head_oid: &str,
    ) -> Result<(), Failure>;
    fn update_branch(
        &self,
        repo: &str,
        number: u64,
        method: UpdateMethod,
    ) -> Result<BranchUpdate, Failure>;
    fn change_status(&self, repo: &str, number: u64, change: &StatusChange) -> Result<(), Failure>;
    fn checkout(&self, repo: &str, number: u64, dir: Option<&Path>) -> Result<(), Failure>;
}

const ENABLE_AUTO_MERGE: &str = "mutation($id: ID!, $method: PullRequestMergeMethod!, $head: GitObjectID) { enablePullRequestAutoMerge(input: {pullRequestId: $id, mergeMethod: $method, expectedHeadOid: $head}) { clientMutationId } }";

const DISABLE_AUTO_MERGE: &str = "mutation($id: ID!) { disablePullRequestAutoMerge(input: {pullRequestId: $id}) { clientMutationId } }";

const ENQUEUE: &str = "mutation($id: ID!, $head: GitObjectID) { enqueuePullRequest(input: {pullRequestId: $id, expectedHeadOid: $head}) { clientMutationId } }";

const DEQUEUE: &str =
    "mutation($id: ID!) { dequeuePullRequest(input: {id: $id}) { clientMutationId } }";

pub const MISSING: &str =
    "gh not found on PATH; install gh (brew install gh) and run gh auth login";

pub const READ_DEADLINE: Duration = Duration::from_secs(30);
pub const WRITE_DEADLINE: Duration = Duration::from_secs(60);

pub struct CliGh {
    program: PathBuf,
    git: PathBuf,
    log: Log,
    read: Duration,
    write: Duration,
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
            git: "git".into(),
            log: Log::default(),
            read: READ_DEADLINE,
            write: WRITE_DEADLINE,
        }
    }

    pub fn with_deadlines(self, read: Duration, write: Duration) -> Self {
        Self {
            read,
            write,
            ..self
        }
    }

    pub fn with_git(self, git: PathBuf) -> Self {
        Self { git, ..self }
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
            StatusChange::Approve => [pr("review"), vec!["--approve".to_string()]].concat(),
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

    pub fn remotes_args() -> Vec<String> {
        ["remote", "-v"].map(str::to_string).to_vec()
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

    fn read(&self, args: &[String]) -> Result<String, Failure> {
        self.exec(&Tool::gh(&self.program), Limit::Read, None, args, &[])
    }

    fn write(&self, args: &[String], aftermath: Aftermath) -> Result<String, Failure> {
        self.exec(
            &Tool::gh(&self.program),
            Limit::Write(aftermath.wording()),
            None,
            args,
            &[],
        )
    }

    fn deadline(&self, limit: Limit) -> Option<Duration> {
        match limit {
            Limit::Read => Some(self.read),
            Limit::Download | Limit::Write(_) => Some(self.write),
            Limit::Unbounded => None,
        }
    }

    fn exec(
        &self,
        tool: &Tool,
        limit: Limit,
        dir: Option<&Path>,
        args: &[String],
        env: &[(&str, &str)],
    ) -> Result<String, Failure> {
        let started = Instant::now();
        let mut command = Command::new(tool.program);
        if let Some(dir) = dir {
            command.current_dir(dir);
        }
        command
            .args(args)
            .envs(env.iter().copied())
            .stdin(Stdio::null());
        let deadline = self.deadline(limit);
        let output = match runner::run(&mut command, deadline) {
            Ok(Ran::Finished(output)) => Ok(output),
            Ok(Ran::TimedOut(stderr)) => {
                self.log
                    .timed_out(tool.name, args, started.elapsed(), &stderr);
                let secs = deadline.unwrap_or_default().as_secs();
                return Err(match limit {
                    Limit::Write(aftermath) => Failure::unanswered(aftermath, secs),
                    _ => Failure::timed_out(format!("{} did not answer within {secs}s", tool.name)),
                });
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                Err(io::Error::new(io::ErrorKind::NotFound, tool.missing))
            }
            Err(err) => Err(err),
        };
        self.log.call(tool.name, args, started.elapsed(), &output);
        let output = output?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                format!("{} exited with {}", tool.name, output.status)
            } else {
                stderr
            };
            return Err((tool.said)(message));
        }
        String::from_utf8(output.stdout)
            .map_err(|err| Failure::from(format!("{} printed invalid UTF-8: {err}", tool.name)))
    }
}

#[derive(Clone, Copy)]
enum Limit {
    Read,
    Download,
    Write(&'static str),
    Unbounded,
}

struct Tool<'a> {
    program: &'a Path,
    name: &'static str,
    missing: &'static str,
    said: fn(String) -> Failure,
}

impl<'a> Tool<'a> {
    fn gh(program: &'a Path) -> Self {
        Self {
            program,
            name: "gh",
            missing: MISSING,
            said: Failure::said_by_gh,
        }
    }

    fn git(program: &'a Path) -> Self {
        Self {
            program,
            name: "git",
            missing: "git not found on PATH",
            said: Failure::from,
        }
    }
}

impl Github for CliGh {
    fn graphql(
        &self,
        query: &str,
        vars: &[(&str, Variable)],
    ) -> Result<serde_json::Value, Failure> {
        let stdout = self.read(&Self::graphql_args(query, vars))?;
        serde_json::from_str(&stdout)
            .map_err(|err| Failure::from(format!("gh returned invalid JSON: {err}")))
    }

    fn rest(&self, path: &str) -> Result<serde_json::Value, Failure> {
        let stdout = self.read(&Self::rest_args(path))?;
        serde_json::from_str(&stdout)
            .map_err(|err| Failure::from(format!("gh returned invalid JSON: {err}")))
    }

    fn current_repo(&self) -> Result<String, Failure> {
        Ok(self.read(&Self::current_repo_args())?.trim().to_string())
    }

    fn log_failed(&self, repo: &str, run_id: u64, job_id: u64) -> Result<String, Failure> {
        self.exec(
            &Tool::gh(&self.program),
            Limit::Download,
            None,
            &Self::log_failed_args(repo, run_id, job_id),
            &[],
        )
    }

    fn rerun_failed(&self, repo: &str, run_id: u64) -> Result<(), Failure> {
        self.write(&Self::rerun_failed_args(repo, run_id), Aftermath::Rerun)
            .map(|_| ())
    }

    fn merge(
        &self,
        repo: &str,
        number: u64,
        method: MergeMethod,
        head_oid: &str,
    ) -> Result<(), Failure> {
        self.write(
            &Self::merge_args(repo, number, method, head_oid),
            Aftermath::Merge,
        )
        .map(|_| ())
        .map_err(without_warnings)
    }

    fn update_branch(
        &self,
        repo: &str,
        number: u64,
        method: UpdateMethod,
    ) -> Result<BranchUpdate, Failure> {
        match self.write(
            &Self::update_branch_args(repo, number, method),
            Aftermath::Update,
        ) {
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

    fn change_status(&self, repo: &str, number: u64, change: &StatusChange) -> Result<(), Failure> {
        self.write(
            &Self::status_args(repo, number, change),
            Aftermath::Status(change),
        )
        .map(|_| ())
        .map_err(without_warnings)
    }

    fn checkout(&self, repo: &str, number: u64, dir: Option<&Path>) -> Result<(), Failure> {
        if let Some(dir) = dir.filter(|dir| !dir.is_dir()) {
            return Err(format!("{} is not a directory", dir.display()).into());
        }
        let remotes = self
            .exec(
                &Tool::git(&self.git),
                Limit::Unbounded,
                dir,
                &Self::remotes_args(),
                &[],
            )
            .map_err(|err| not_a_repository(err, repo, dir))?;
        if !remotes.lines().any(|remote| names(remote, repo)) {
            return Err(no_clone(repo, dir));
        }
        self.exec(
            &Tool::gh(&self.program),
            Limit::Unbounded,
            dir,
            &Self::checkout_args(repo, number),
            &[("GIT_TERMINAL_PROMPT", "0")],
        )
        .map(|_| ())
        .map_err(gits_reason)
    }
}

fn names(remote: &str, repo: &str) -> bool {
    let Some(url) = remote.split_whitespace().nth(1) else {
        return false;
    };
    let url = url.trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let mut segments = url.rsplit(['/', ':']);
    let (Some(name), Some(owner)) = (segments.next(), segments.next()) else {
        return false;
    };
    format!("{owner}/{name}").eq_ignore_ascii_case(repo)
}

fn not_a_repository(err: Failure, repo: &str, dir: Option<&Path>) -> Failure {
    if err.message().contains("not a git repository") {
        return no_clone(repo, dir);
    }
    gits_reason(err)
}

fn no_clone(repo: &str, dir: Option<&Path>) -> Failure {
    Failure::from(match dir {
        Some(dir) => format!("{} is not a clone of {repo}", dir.display()),
        None => {
            format!("conveyor does not run in a clone of {repo}; set `path` under its [[repo]]")
        }
    })
}

fn gits_reason(err: Failure) -> Failure {
    let reason = err
        .message()
        .lines()
        .find(|line| line.starts_with("fatal: ") || line.starts_with("error: "))
        .or_else(|| err.message().lines().rfind(|line| !line.trim().is_empty()))
        .map(str::to_string);
    match reason {
        Some(line) => err.reworded(line),
        None => err,
    }
}

pub enum Aftermath<'a> {
    Merge,
    Update,
    Rerun,
    Status(&'a StatusChange),
}

impl Aftermath<'_> {
    pub fn wording(&self) -> &'static str {
        match self {
            Self::Merge => "may be merged",
            Self::Update => "branch may be updated",
            Self::Rerun => "may be rerunning",
            Self::Status(StatusChange::Ready) => "may be marked ready",
            Self::Status(StatusChange::Draft) => "may be a draft",
            Self::Status(StatusChange::EnableAutoMerge { .. }) => "auto-merge may be on",
            Self::Status(StatusChange::DisableAutoMerge { .. }) => "auto-merge may be off",
            Self::Status(StatusChange::Enqueue { .. }) => "may be in the queue",
            Self::Status(StatusChange::Dequeue { .. }) => "may be out of the queue",
            Self::Status(StatusChange::Close) => "may be closed",
            Self::Status(StatusChange::Approve) => "may be approved",
        }
    }
}

fn pinned<'a>(mut vars: Vec<(&'a str, Variable)>, head_oid: &str) -> Vec<(&'a str, Variable)> {
    if !head_oid.is_empty() {
        vars.push(("head", head_oid.into()));
    }
    vars
}

fn without_warnings(err: Failure) -> Failure {
    let kept: Vec<&str> = err
        .message()
        .lines()
        .filter(|line| !line.starts_with("! "))
        .collect();
    if kept.is_empty() {
        return err;
    }
    let kept = kept.join("\n");
    err.reworded(kept)
}
