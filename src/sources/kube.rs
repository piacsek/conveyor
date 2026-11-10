use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::DeployEnv;
use crate::log::Log;

pub const DEADLINE: Duration = Duration::from_secs(15);
const POLL: Duration = Duration::from_millis(10);
const DRAIN: Duration = Duration::from_secs(1);
pub const MISSING: &str = "kubectl not found on PATH; install kubectl and log in to the cluster";

pub trait Kube {
    fn deployment(&self, env: &DeployEnv) -> io::Result<String>;
    fn pods(&self, env: &DeployEnv, selector: &str) -> io::Result<String>;
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

    fn run(&self, args: &[String]) -> io::Result<String> {
        let started = Instant::now();
        let output = self
            .output(Command::new(&self.program).args(args).stdin(Stdio::null()))
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

    fn output(&self, command: &mut Command) -> io::Result<Output> {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() >= self.deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(self.timed_out());
            }
            thread::sleep(POLL);
        };
        let until = Instant::now() + self.deadline.saturating_sub(started.elapsed()).max(DRAIN);
        let left = || until.saturating_duration_since(Instant::now());
        match (stdout.recv_timeout(left()), stderr.recv_timeout(left())) {
            (Ok(stdout), Ok(stderr)) => Ok(Output {
                status,
                stdout,
                stderr,
            }),
            _ => Err(self.timed_out()),
        }
    }

    fn timed_out(&self) -> io::Error {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("kubectl did not answer within {}s", self.deadline.as_secs()),
        )
    }
}

fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = sender.send(bytes);
    });
    receiver
}

impl Kube for CliKubectl {
    fn deployment(&self, env: &DeployEnv) -> io::Result<String> {
        self.run(&Self::deployment_args(env))
    }

    fn pods(&self, env: &DeployEnv, selector: &str) -> io::Result<String> {
        self.run(&Self::pods_args(env, selector))
    }
}
