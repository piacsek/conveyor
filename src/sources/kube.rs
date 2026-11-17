use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::DeployEnv;
use crate::log::Log;
use crate::sources::failure::Failure;
use crate::sources::runner::{self, Ran};

pub const DEADLINE: Duration = Duration::from_secs(15);
pub const MISSING: &str = "kubectl not found on PATH; install kubectl and log in to the cluster";

pub trait Kube {
    fn deployment(&self, env: &DeployEnv) -> Result<String, Failure>;
    fn pods(&self, env: &DeployEnv, selector: &str) -> Result<String, Failure>;
}

pub struct CliKubectl {
    program: PathBuf,
    log: Log,
    deadline: Duration,
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
            deadline: DEADLINE,
        }
    }

    pub fn with_log(self, log: Log) -> Self {
        Self { log, ..self }
    }

    pub fn with_deadline(self, deadline: Duration) -> Self {
        Self { deadline, ..self }
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

    fn run(&self, args: &[String]) -> Result<String, Failure> {
        let started = Instant::now();
        let ran = runner::run(
            Command::new(&self.program).args(args).stdin(Stdio::null()),
            Some(self.deadline),
        );
        let output = match ran {
            Ok(Ran::Finished(output)) => Ok(output),
            Ok(Ran::TimedOut(stderr)) => {
                self.log
                    .timed_out("kubectl", args, started.elapsed(), &stderr);
                return Err(Failure::timed_out(format!(
                    "kubectl did not answer within {}s",
                    self.deadline.as_secs()
                )));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                Err(io::Error::new(io::ErrorKind::NotFound, MISSING))
            }
            Err(err) => Err(err),
        };
        self.log.call("kubectl", args, started.elapsed(), &output);
        let output = output?;
        if !output.status.success() {
            let stderr = said(&String::from_utf8_lossy(&output.stderr));
            let message = if stderr.is_empty() {
                format!("kubectl exited with {}", output.status)
            } else {
                stderr
            };
            return Err(Failure::from(message));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

fn said(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().filter(|line| !is_klog(line)).collect();
    let said = lines.join("\n").trim().to_string();
    if said.is_empty() {
        stderr.trim().to_string()
    } else {
        said
    }
}

fn is_klog(line: &str) -> bool {
    let mut chars = line.chars();
    matches!(chars.next(), Some('E' | 'W' | 'I' | 'F'))
        && chars.by_ref().take(4).filter(char::is_ascii_digit).count() == 4
        && chars.next() == Some(' ')
        && line.contains(".go:")
}

impl Kube for CliKubectl {
    fn deployment(&self, env: &DeployEnv) -> Result<String, Failure> {
        self.run(&Self::deployment_args(env))
    }

    fn pods(&self, env: &DeployEnv, selector: &str) -> Result<String, Failure> {
        self.run(&Self::pods_args(env, selector))
    }
}
