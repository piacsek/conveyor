use std::path::Path;
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::config::{Config, DeployEnv, MergeMethod, UpdateMethod};
use crate::model::prs::{BranchUpdate, StatusChange};
use crate::sources::failure::Failure;
use crate::sources::fetch::{PRS_QUERY, QUEUE_QUERY};
use crate::sources::github::{Github, Variable};
use crate::sources::kube::Kube;

const PRS: &str = include_str!("demo/prs.json");
const QUEUE: &str = include_str!("demo/queue.json");
const RUNS: &str = include_str!("demo/runs.json");
const MERGE_GROUP_RUNS: &str = include_str!("demo/merge-group-runs.json");
const WORKFLOWS: &str = include_str!("demo/workflows.json");
const PULLS: &str = include_str!("demo/pulls.json");
const JOBS: &str = include_str!("demo/jobs.json");
const DEPLOYMENTS: &str = include_str!("demo/deployments.json");
const PODS: &str = include_str!("demo/pods.json");
const CONFIG: &str = include_str!("demo/config.toml");
const EXPIRED: &str = "ERROR: Active profile expired.";
const NO_COMMITS: &str = r#"{"data":{"repository":{}}}"#;
const EMPTY_QUEUE: &str = r#"{"data":{"repository":{"mergeQueue":{"url":"https://github.com/acme/infra/queue/main","entries":{"nodes":[]}}}}}"#;

pub fn config(config: Config) -> Config {
    let demo: Config = toml::from_str(CONFIG).unwrap_or_default();
    Config {
        prs: demo.prs,
        repo: demo.repo,
        ..config
    }
}

#[derive(Default)]
pub struct Demo;

impl Demo {
    fn answer(&self, fixture: &str) -> Result<Value, Failure> {
        let mut value: Value = serde_json::from_str(fixture)
            .map_err(|err| Failure::from(format!("demo fixture is invalid JSON: {err}")))?;
        aged(&mut value, SystemTime::now());
        Ok(value)
    }

    fn environment(&self, fixture: &str, env: &DeployEnv) -> Result<String, Failure> {
        let answer = self.answer(fixture)?;
        answer
            .get(&env.name)
            .map(Value::to_string)
            .ok_or_else(|| Failure::from(EXPIRED.to_string()))
    }
}

fn aged(value: &mut Value, now: SystemTime) {
    match value {
        Value::String(text) => {
            if let Some(secs) = text.strip_prefix("ago:").and_then(|s| s.parse().ok()) {
                let at = chrono::DateTime::<chrono::Utc>::from(now - Duration::from_secs(secs));
                *text = at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| aged(item, now)),
        Value::Object(fields) => fields.values_mut().for_each(|field| aged(field, now)),
        _ => {}
    }
}

fn jobs(run: &str) -> String {
    let (status, conclusion, completed) = match run {
        "1" | "311" => ("completed", "\"failure\"", "\"ago:1181\""),
        "2" | "312" | "313" => ("in_progress", "null", "null"),
        _ => ("completed", "\"success\"", "\"ago:1181\""),
    };
    JOBS.replace("STATUS", status)
        .replace("CONCLUSION", conclusion)
        .replace("COMPLETED", completed)
        .replace("RUN", run)
}

fn no_fixture(what: &str) -> Failure {
    Failure::from(format!("demo: no fixture for {what}"))
}

impl Github for Demo {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> Result<Value, Failure> {
        match query {
            PRS_QUERY => self.answer(PRS),
            QUEUE_QUERY if vars.iter().any(|(_, v)| *v == Variable::from("webapp")) => {
                self.answer(QUEUE)
            }
            QUEUE_QUERY => self.answer(EMPTY_QUEUE),
            _ => self.answer(NO_COMMITS),
        }
    }

    fn rest(&self, path: &str) -> Result<Value, Failure> {
        let (route, query) = path.split_once('?').unwrap_or((path, ""));
        match route.split('/').collect::<Vec<_>>().as_slice() {
            [_, _, _, "actions", "workflows"] => self.answer(WORKFLOWS),
            [_, _, _, "actions", "workflows", _, "runs"] => self.answer(RUNS),
            [_, _, _, "actions", "runs"] if query.contains("event=merge_group") => {
                self.answer(MERGE_GROUP_RUNS)
            }
            [_, _, _, "actions", "runs", run, "jobs"] => self.answer(&jobs(run)),
            [_, _, _, "commits", sha, "pulls"] => {
                let pulls = self.answer(PULLS)?;
                Ok(pulls.get(*sha).cloned().unwrap_or(Value::Array(vec![])))
            }
            _ => Err(no_fixture(route)),
        }
    }

    fn current_repo(&self) -> Result<String, Failure> {
        Err(no_fixture("the current repository"))
    }

    fn log_failed(&self, _repo: &str, _run_id: u64, _job_id: u64) -> Result<String, Failure> {
        Err(no_fixture("the log"))
    }

    fn rerun_failed(&self, _repo: &str, _run_id: u64) -> Result<(), Failure> {
        Err(no_fixture("a rerun"))
    }

    fn merge(&self, _: &str, _: u64, _: MergeMethod, _: &str) -> Result<(), Failure> {
        Err(no_fixture("a merge"))
    }

    fn update_branch(&self, _: &str, _: u64, _: UpdateMethod) -> Result<BranchUpdate, Failure> {
        Err(no_fixture("an update"))
    }

    fn change_status(&self, _: &str, _: u64, _: &StatusChange) -> Result<(), Failure> {
        Err(no_fixture("a status change"))
    }

    fn checkout(&self, _: &str, _: u64, _: Option<&Path>) -> Result<(), Failure> {
        Err(no_fixture("a checkout"))
    }
}

impl Kube for Demo {
    fn deployment(&self, env: &DeployEnv) -> Result<String, Failure> {
        self.environment(DEPLOYMENTS, env)
    }

    fn pods(&self, env: &DeployEnv, _selector: &str) -> Result<String, Failure> {
        self.environment(PODS, env)
    }
}
