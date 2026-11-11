use std::cell::RefCell;
use std::io;

use conveyor::config::{MergeMethod, UpdateMethod};
use conveyor::model::prs::{BranchUpdate, StatusChange};
use conveyor::sources::github::{Github, Variable};

pub const NOW: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

pub type Call = (String, Vec<(String, Variable)>);

pub struct FakeGithub {
    pub calls: RefCell<Vec<Call>>,
    pub response: io::Result<serde_json::Value>,
    pub rest_response: io::Result<serde_json::Value>,
    pub rest_routes: Vec<(String, serde_json::Value)>,
    pub graphql_routes: Vec<(String, serde_json::Value)>,
    pub cwd_repo: io::Result<String>,
    pub log_response: io::Result<String>,
    pub rerun_failures: Vec<(u64, String)>,
}

impl FakeGithub {
    pub fn with_response(response: io::Result<serde_json::Value>) -> Self {
        Self {
            calls: RefCell::default(),
            response,
            rest_response: Ok(serde_json::json!({"workflow_runs": []})),
            rest_routes: Vec::new(),
            graphql_routes: Vec::new(),
            cwd_repo: Ok("acme/from-cwd".to_string()),
            log_response: Ok(String::new()),
            rerun_failures: Vec::new(),
        }
    }
}

impl Github for FakeGithub {
    fn graphql(&self, query: &str, vars: &[(&str, Variable)]) -> io::Result<serde_json::Value> {
        self.calls.borrow_mut().push((
            query.to_string(),
            vars.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        if let Some((_, value)) = self
            .graphql_routes
            .iter()
            .find(|(needle, _)| query.contains(needle.as_str()))
        {
            return Ok(value.clone());
        }
        match &self.response {
            Ok(value) => Ok(value.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }

    fn rest(&self, path: &str) -> io::Result<serde_json::Value> {
        self.calls
            .borrow_mut()
            .push((format!("GET {path}"), Vec::new()));
        if let Some((_, value)) = self
            .rest_routes
            .iter()
            .find(|(prefix, _)| path.starts_with(prefix))
        {
            return Ok(value.clone());
        }
        match &self.rest_response {
            Ok(value) => Ok(value.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }

    fn current_repo(&self) -> io::Result<String> {
        match &self.cwd_repo {
            Ok(name) => Ok(name.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }

    fn log_failed(&self, repo: &str, run_id: u64, job_id: u64) -> io::Result<String> {
        self.calls
            .borrow_mut()
            .push((format!("LOG {repo} {run_id} {job_id}"), Vec::new()));
        match &self.log_response {
            Ok(text) => Ok(text.clone()),
            Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
        }
    }

    fn rerun_failed(&self, repo: &str, run_id: u64) -> io::Result<()> {
        self.calls
            .borrow_mut()
            .push((format!("RERUN {repo} {run_id}"), Vec::new()));
        match self.rerun_failures.iter().find(|(id, _)| *id == run_id) {
            Some((_, message)) => Err(io::Error::other(message.clone())),
            None => Ok(()),
        }
    }

    fn merge(
        &self,
        repo: &str,
        number: u64,
        method: MergeMethod,
        head_oid: &str,
    ) -> io::Result<()> {
        self.calls.borrow_mut().push((
            format!("MERGE {repo} {number} {method:?} {head_oid}"),
            Vec::new(),
        ));
        Ok(())
    }

    fn update_branch(
        &self,
        repo: &str,
        number: u64,
        method: UpdateMethod,
    ) -> io::Result<BranchUpdate> {
        self.calls
            .borrow_mut()
            .push((format!("UPDATE {repo} {number} {method:?}"), Vec::new()));
        Ok(BranchUpdate::Updated)
    }

    fn change_status(&self, repo: &str, number: u64, change: &StatusChange) -> io::Result<()> {
        self.calls
            .borrow_mut()
            .push((format!("STATUS {repo} {number} {change:?}"), Vec::new()));
        Ok(())
    }

    fn checkout(&self, repo: &str, number: u64, _dir: Option<&std::path::Path>) -> io::Result<()> {
        self.calls
            .borrow_mut()
            .push((format!("CHECKOUT {repo} {number}"), Vec::new()));
        Ok(())
    }
}

pub struct FakeKube {
    pub deployments: Vec<(String, io::Result<String>)>,
    pub pods: Result<String, String>,
    pub selectors: std::sync::Mutex<Vec<String>>,
}

impl FakeKube {
    pub fn returning(image: &str) -> Self {
        Self::with(vec![("prod".to_string(), Ok(deployment_json(image)))])
    }

    pub fn with(deployments: Vec<(String, io::Result<String>)>) -> Self {
        Self {
            deployments,
            pods: Ok(r#"{"items":[]}"#.to_string()),
            selectors: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl conveyor::sources::kube::Kube for FakeKube {
    fn deployment(&self, env: &conveyor::config::DeployEnv) -> io::Result<String> {
        match self.deployments.iter().find(|(name, _)| *name == env.name) {
            Some((_, Ok(json))) => Ok(json.clone()),
            Some((_, Err(err))) => Err(io::Error::new(err.kind(), err.to_string())),
            None => Err(io::Error::other("no such env")),
        }
    }

    fn pods(&self, _env: &conveyor::config::DeployEnv, selector: &str) -> io::Result<String> {
        self.selectors.lock().unwrap().push(selector.to_string());
        self.pods.clone().map_err(io::Error::other)
    }
}

pub fn deploy_source_for(
    sha: &str,
    routes: Vec<(String, serde_json::Value)>,
) -> (FakeGithub, FakeKube, conveyor::sources::fetch::DeploySource) {
    use conveyor::config::{Deploy, DeployEnv};
    use conveyor::sources::fetch::DeploySource;
    let mut gh = FakeGithub::with_response(Ok(serde_json::Value::Null));
    gh.rest_routes = routes;
    let deploy = Deploy {
        env: vec![DeployEnv {
            name: "prod".to_string(),
            ..DeployEnv::default()
        }],
        ..Deploy::default()
    };
    (
        gh,
        FakeKube::returning(&format!("ghcr.io/acme/api:{sha}")),
        DeploySource::new("acme/webapp".to_string(), deploy),
    )
}

pub const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

pub fn deployment_json(image: &str) -> String {
    serde_json::json!({
        "spec": {
            "replicas": 1,
            "selector": {"matchLabels": {"app": "api"}},
            "template": {"spec": {"containers": [{"image": image}]}}
        },
        "status": {
            "replicas": 1,
            "updatedReplicas": 1,
            "readyReplicas": 1,
            "availableReplicas": 1,
            "conditions": [{"type": "Available", "status": "True"}]
        }
    })
    .to_string()
}
