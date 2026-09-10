use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::config::DeployEnv;

pub const MISSING: &str = "kubectl not found on PATH; install kubectl and log in to the cluster";

pub trait Kube {
    fn image(&self, env: &DeployEnv) -> io::Result<String>;
}

pub struct CliKubectl {
    program: PathBuf,
}

impl Default for CliKubectl {
    fn default() -> Self {
        Self::with_program("kubectl".into())
    }
}

impl CliKubectl {
    pub fn with_program(program: PathBuf) -> Self {
        Self { program }
    }

    pub fn image_args(env: &DeployEnv) -> Vec<String> {
        [
            "--context",
            env.context.as_str(),
            "-n",
            env.namespace.as_str(),
            "get",
            "deploy",
            env.deployment.as_str(),
            "-o",
            "jsonpath={.spec.template.spec.containers[0].image}",
            "--request-timeout=10s",
        ]
        .map(str::to_string)
        .to_vec()
    }
}

impl Kube for CliKubectl {
    fn image(&self, env: &DeployEnv) -> io::Result<String> {
        let output = Command::new(&self.program)
            .args(Self::image_args(env))
            .stdin(Stdio::null())
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
                format!("kubectl exited with {}", output.status)
            } else {
                stderr
            };
            return Err(io::Error::other(message));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
