use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::config::DeployEnv;
use crate::log::Log;

pub const MISSING: &str = "kubectl not found on PATH; install kubectl and log in to the cluster";

pub trait Kube {
    fn deployment(&self, env: &DeployEnv) -> io::Result<String>;
    fn pods(&self, env: &DeployEnv, selector: &str) -> io::Result<String>;
}

pub struct CliKubectl {
    program: PathBuf,
    log: Log,
}

impl Default for CliKubectl {
    fn default() -> Self {
        Self::with_program("kubectl".into())
    }
}

impl CliKubectl {
    pub fn with_program(program: PathBuf) -> Self {
        Self {
            program,
            log: Log::default(),
        }
    }

    pub fn with_log(self, log: Log) -> Self {
        Self { log, ..self }
    }

    pub fn deployment_args(env: &DeployEnv) -> Vec<String> {
        Self::get_args(env, &["deploy", env.deployment.as_str()])
    }

    pub fn pods_args(env: &DeployEnv, selector: &str) -> Vec<String> {
        Self::get_args(env, &["pods", "-l", selector])
    }

    fn get_args(env: &DeployEnv, what: &[&str]) -> Vec<String> {
        [
            "--context",
            env.context.as_str(),
            "-n",
            env.namespace.as_str(),
            "get",
        ]
        .iter()
        .chain(what)
        .chain(&["-o", "json", "--request-timeout=10s"])
        .map(|arg| arg.to_string())
        .collect()
    }

    fn run(&self, args: &[String]) -> io::Result<String> {
        let started = Instant::now();
        let output = Command::new(&self.program)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_err(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    io::Error::new(io::ErrorKind::NotFound, MISSING)
                } else {
                    err
                }
            });
        self.log.call("kubectl", args, started.elapsed(), &output);
        let output = output?;
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

impl Kube for CliKubectl {
    fn deployment(&self, env: &DeployEnv) -> io::Result<String> {
        self.run(&Self::deployment_args(env))
    }

    fn pods(&self, env: &DeployEnv, selector: &str) -> io::Result<String> {
        self.run(&Self::pods_args(env, selector))
    }
}
