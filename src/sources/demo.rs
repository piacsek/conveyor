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
const REVIEW: &str = include_str!("demo/review.json");
const QUEUE: &str = include_str!("demo/queue.json");
const RUNS: &str = include_str!("demo/runs.json");
const INFRA_RUNS: &str = include_str!("demo/infra-runs.json");
const MERGE_GROUP_RUNS: &str = include_str!("demo/merge-group-runs.json");
const WORKFLOWS: &str = include_str!("demo/workflows.json");
const PULLS: &str = include_str!("demo/pulls.json");
const JOBS: &str = include_str!("demo/jobs.json");
const DEPLOYMENTS: &str = include_str!("demo/deployments.json");
const PODS: &str = include_str!("demo/pods.json");
const FAILED_LOG: &str = include_str!("demo/failed.log");
const SESSIONS_LOG: &str = include_str!("demo/sessions.log");
const CONFIG: &str = include_str!("demo/config.toml");
const EXPIRED: &str = "ERROR: Active profile expired.";
const REPOSITORY: &str = r#"{"default_branch":"main"}"#;
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
    let (started, finished, failed) = match run {
        "1" => (1800, Some(1560), true),
        "311" => (3600, Some(2503), true),
        "2" => (2400, None, false),
        "4" => (720, None, false),
        "312" => (95, None, false),
        "313" => (480, None, false),
        "314" => (240, None, false),
        "5" => (6000, Some(5520), false),
        "6" => (87000, Some(86700), false),
        "7" => (2400, Some(1980), false),
        "309" => (90000, Some(89070), false),
        "310" => (7200, Some(6198), false),
        _ => (100000, Some(99000), false),
    };
    let (status, conclusion, completed) = match finished {
        None => ("in_progress", "null".to_string(), "null".to_string()),
        Some(end) if failed => (
            "completed",
            "\"failure\"".to_string(),
            format!("\"ago:{end}\""),
        ),
        Some(end) => (
            "completed",
            "\"success\"".to_string(),
            format!("\"ago:{end}\""),
        ),
    };
    JOBS.replace("STATUS", status)
        .replace("CONCLUSION", &conclusion)
        .replace("COMPLETED", &completed)
        .replace("STARTED", &started.to_string())
        .replace("AUDITED", &(started - 60).to_string())
        .replace("RUN", run)
}

fn asks(vars: &[(&str, Variable)], term: &str) -> bool {
    vars.iter().any(|(name, value)| {
        *name == "q" && matches!(value, Variable::Text(query) if query.contains(term))
    })
}

fn nothing_changed() -> Failure {
    Failure::from("demo: nothing was changed".to_string())
}

fn no_fixture(what: &str) -> Failure {
    Failure::from(format!("demo: no fixture for {what}"))
}

impl Github for Demo {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> Result<Value, Failure> {
        match query {
            PRS_QUERY if asks(vars, "review-requested") => self.answer(REVIEW),
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
            [_, _, _] => self.answer(REPOSITORY),
            [_, _, _, "actions", "workflows"] => self.answer(WORKFLOWS),
            [_, _, "webapp", "actions", "workflows", _, "runs"] => self.answer(RUNS),
            [_, _, "infra", "actions", "workflows", _, "runs"] => self.answer(INFRA_RUNS),
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

    fn log_failed(&self, _repo: &str, run_id: u64, _job_id: u64) -> Result<String, Failure> {
        match run_id {
            311 => Ok(SESSIONS_LOG.to_string()),
            _ => Ok(FAILED_LOG.to_string()),
        }
    }

    fn rerun_failed(&self, _repo: &str, _run_id: u64) -> Result<(), Failure> {
        Err(nothing_changed())
    }

    fn merge(&self, _: &str, _: u64, _: MergeMethod, _: &str) -> Result<(), Failure> {
        Err(nothing_changed())
    }

    fn update_branch(&self, _: &str, _: u64, _: UpdateMethod) -> Result<BranchUpdate, Failure> {
        Err(nothing_changed())
    }

    fn change_status(&self, _: &str, _: u64, _: &StatusChange) -> Result<(), Failure> {
        Err(nothing_changed())
    }

    fn checkout(&self, _: &str, _: u64, _: Option<&Path>) -> Result<(), Failure> {
        Err(nothing_changed())
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
