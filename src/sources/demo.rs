use std::path::Path;
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::config::{DeployEnv, MergeMethod, UpdateMethod};
use crate::model::prs::{BranchUpdate, StatusChange};
use crate::sources::failure::Failure;
use crate::sources::fetch::PRS_QUERY;
use crate::sources::github::{Github, Variable};
use crate::sources::kube::Kube;

const PRS: &str = include_str!("demo/prs.json");

#[derive(Default)]
pub struct Demo;

impl Demo {
    fn answer(&self, fixture: &str) -> Result<Value, Failure> {
        let mut value: Value = serde_json::from_str(fixture)
            .map_err(|err| Failure::from(format!("demo fixture is invalid JSON: {err}")))?;
        aged(&mut value, SystemTime::now());
        Ok(value)
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

fn no_fixture(what: &str) -> Failure {
    Failure::from(format!("demo: no fixture for {what}"))
}

impl Github for Demo {
    fn graphql(&self, query: &str, _vars: &[(&str, Variable)]) -> Result<Value, Failure> {
        if query == PRS_QUERY {
            return self.answer(PRS);
        }
        Err(no_fixture("this query"))
    }

    fn rest(&self, path: &str) -> Result<Value, Failure> {
        Err(no_fixture(path))
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
        Err(no_fixture(&env.name))
    }

    fn pods(&self, env: &DeployEnv, _selector: &str) -> Result<String, Failure> {
        Err(no_fixture(&env.name))
    }
}
