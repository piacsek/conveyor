use crate::config::{Config, Repo};
use crate::model::builds::Builds;
use crate::model::prs::PullRequest;
use crate::model::queue::{Queue, parse_merge_group_runs};
use crate::sources::github::Github;

pub const PRS_QUERY: &str = include_str!("queries/prs.graphql");
pub const QUEUE_QUERY: &str = include_str!("queries/queue.graphql");
const QUEUE_LIMIT: usize = 20;

pub fn fetch_prs(gh: &impl Github, config: &Config) -> Result<Vec<PullRequest>, String> {
    let value = gh
        .graphql(
            PRS_QUERY,
            &[
                ("q", config.prs.query.as_str().into()),
                ("first", config.prs.limit.into()),
            ],
        )
        .map_err(|err| err.to_string())?;
    crate::model::prs::parse(&value)
}

pub fn fetch_jobs(
    gh: &impl Github,
    repo: &str,
    run_id: u64,
) -> Result<Vec<crate::model::jobs::Job>, String> {
    let value = gh
        .rest(&format!(
            "repos/{repo}/actions/runs/{run_id}/jobs?per_page=100"
        ))
        .map_err(|err| err.to_string())?;
    Ok(crate::model::jobs::parse_jobs(&value))
}

pub fn repos(gh: &impl Github, config: &Config) -> Result<Vec<Repo>, String> {
    if !config.repo.is_empty() {
        return Ok(config.repo.clone());
    }
    let name = gh.current_repo().map_err(|err| {
        format!("no [[repo]] configured and the current directory has none: {err}")
    })?;
    Ok(vec![Repo {
        name,
        ..Repo::default()
    }])
}

pub fn fetch_queue(gh: &impl Github, repo: &Repo) -> Result<Queue, String> {
    let (owner, name) = repo
        .name
        .split_once('/')
        .ok_or_else(|| format!("repo `{}` must be owner/name", repo.name))?;
    let value = gh
        .graphql(
            QUEUE_QUERY,
            &[
                ("owner", owner.into()),
                ("name", name.into()),
                ("first", QUEUE_LIMIT.into()),
            ],
        )
        .map_err(|err| err.to_string())?;
    let mut queue = crate::model::queue::parse(&value)?;
    queue.repo = repo.name.clone();
    if !queue.entries.is_empty()
        && let Ok(runs) = gh.rest(&format!(
            "repos/{}/actions/runs?event=merge_group&per_page=30",
            repo.name
        ))
    {
        queue.attach_runs(&parse_merge_group_runs(&runs));
    }
    Ok(queue)
}

const DEFAULT_WORKFLOWS: [&str; 8] = [
    "CI/CD",
    "cicd.yml",
    "CI",
    "ci",
    "ci.yml",
    "build",
    "build.yml",
    "main.yml",
];

pub struct Pulls {
    repo: String,
    cache: std::collections::HashMap<String, crate::model::builds::PullRef>,
}

impl Pulls {
    pub fn new(repo: String) -> Self {
        Self {
            repo,
            cache: std::collections::HashMap::new(),
        }
    }

    pub fn for_sha(
        &mut self,
        gh: &impl Github,
        sha: &str,
    ) -> Option<crate::model::builds::PullRef> {
        if let Some(cached) = self.cache.get(sha) {
            return Some(cached.clone());
        }
        let pull = self
            .associated(gh, sha)
            .or_else(|| self.by_squash_suffix(gh, sha))?;
        self.cache.insert(sha.to_string(), pull.clone());
        Some(pull)
    }

    fn associated(&self, gh: &impl Github, sha: &str) -> Option<crate::model::builds::PullRef> {
        let value = gh
            .rest(&format!("repos/{}/commits/{sha}/pulls", self.repo))
            .ok()?;
        crate::model::builds::parse_pull_numbers(&value)
    }

    fn by_squash_suffix(
        &self,
        gh: &impl Github,
        sha: &str,
    ) -> Option<crate::model::builds::PullRef> {
        let commit = gh
            .rest(&format!("repos/{}/commits/{sha}", self.repo))
            .ok()?;
        let message = commit.pointer("/commit/message")?.as_str()?;
        let number = crate::model::builds::pr_number_from_title(message.lines().next()?)?;
        let pull = gh
            .rest(&format!("repos/{}/pulls/{number}", self.repo))
            .ok()?;
        crate::model::builds::parse_pull(&pull)
    }
}

pub struct BuildsSource {
    repo: Repo,
    workflow: Option<String>,
    pulls: Pulls,
}

impl BuildsSource {
    pub fn new(repo: Repo) -> Self {
        Self {
            pulls: Pulls::new(repo.name.clone()),
            repo,
            workflow: None,
        }
    }

    pub fn fetch(&mut self, gh: &impl Github) -> Result<Vec<crate::model::builds::Build>, String> {
        let workflow = self.workflow(gh)?;
        let runs = gh
            .rest(&format!(
                "repos/{}/actions/workflows/{workflow}/runs?branch=main&event=push&per_page={}",
                self.repo.name, self.repo.builds
            ))
            .map_err(|err| err.to_string())?;
        let mut builds = crate::model::builds::parse_runs(&runs);
        for build in &mut builds {
            build.pull = self.pulls.for_sha(gh, &build.sha);
        }
        Ok(builds)
    }

    pub fn into_builds(&self, builds: Vec<crate::model::builds::Build>) -> Builds {
        Builds {
            repo: self.repo.name.clone(),
            builds,
        }
    }

    fn workflow(&mut self, gh: &impl Github) -> Result<String, String> {
        if let Some(workflow) = &self.workflow {
            return Ok(workflow.clone());
        }
        let wanted = self.repo.main_workflow.as_str();
        let resolved = if wanted.ends_with(".yml") || wanted.ends_with(".yaml") {
            wanted.to_string()
        } else {
            let list = gh
                .rest(&format!(
                    "repos/{}/actions/workflows?per_page=100",
                    self.repo.name
                ))
                .map_err(|err| err.to_string())?;
            let candidates: Vec<&str> = if wanted == Repo::default().main_workflow {
                DEFAULT_WORKFLOWS.to_vec()
            } else {
                vec![wanted]
            };
            list.get("workflows")
                .and_then(|v| v.as_array())
                .and_then(|workflows| {
                    candidates.iter().find_map(|candidate| {
                        workflows.iter().find(|w| {
                            w.get("name").and_then(|v| v.as_str()) == Some(candidate)
                                || w.get("path")
                                    .and_then(|v| v.as_str())
                                    .is_some_and(|path| path.ends_with(&format!("/{candidate}")))
                        })
                    })
                })
                .and_then(|w| w.get("id"))
                .and_then(|v| v.as_u64())
                .map(|id| id.to_string())
                .ok_or_else(|| {
                    format!(
                        "main_workflow `{wanted}` not found in {}; use the workflow name or file",
                        self.repo.name
                    )
                })?
        };
        self.workflow = Some(resolved.clone());
        Ok(resolved)
    }
}

pub struct DeploySource {
    deploy: crate::config::Deploy,
    pulls: Pulls,
}

impl DeploySource {
    pub fn new(repo: String, deploy: crate::config::Deploy) -> Self {
        Self {
            deploy,
            pulls: Pulls::new(repo),
        }
    }

    pub fn fetch(
        &mut self,
        gh: &impl Github,
        kube: &impl crate::sources::kube::Kube,
        now: std::time::SystemTime,
    ) -> crate::model::deployed::Deployed {
        let rows = self
            .deploy
            .env
            .clone()
            .iter()
            .map(|env| self.fetch_env(gh, kube, env, now))
            .collect();
        crate::model::deployed::Deployed {
            system: self.deploy.system.clone(),
            rows,
        }
    }

    fn fetch_env(
        &mut self,
        gh: &impl Github,
        kube: &impl crate::sources::kube::Kube,
        env: &crate::config::DeployEnv,
        now: std::time::SystemTime,
    ) -> crate::model::deployed::Deployment {
        let mut row = crate::model::deployed::Deployment {
            env: env.name.clone(),
            ..Default::default()
        };
        let image = match kube.image(env) {
            Ok(image) => image,
            Err(err) => {
                row.error = Some(err.to_string());
                return row;
            }
        };
        row.image = Some(image.clone());
        match crate::model::deployed::sha_from_image(&image) {
            Ok(sha) => {
                row.pull = self.pulls.for_sha(gh, &sha);
                row.sha = Some(sha);
                row.fetched_at = Some(now);
            }
            Err(err) => row.error = Some(err),
        }
        row
    }
}
