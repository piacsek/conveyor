use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::DeployEnv;
use crate::log::Log;

pub const DEADLINE: Duration = Duration::from_secs(15);
const POLL: Duration = Duration::from_millis(10);
const DRAIN: Duration = Duration::from_millis(500);
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
        let ran = self.output(Command::new(&self.program).args(args).stdin(Stdio::null()));
        let output = match ran {
            Ok(Ran::Finished(output)) => Ok(output),
            Ok(Ran::TimedOut(stderr)) => {
                self.log
                    .timed_out("kubectl", args, started.elapsed(), &stderr);
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("kubectl did not answer within {}s", self.deadline.as_secs()),
                ));
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
            return Err(io::Error::other(message));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn output(&self, command: &mut Command) -> io::Result<Ran> {
        let mut child = command
            .process_group(0)
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
                kill_group(child.id());
                let _ = child.wait();
                return Ok(Ran::TimedOut(
                    stderr.recv_timeout(DRAIN).unwrap_or_default(),
                ));
            }
            thread::sleep(POLL);
        };
        let mut answer = within(DRAIN, &stdout, &stderr);
        if answer.0.is_err() || answer.1.is_err() {
            kill_group(child.id());
            let after = within(DRAIN, &stdout, &stderr);
            answer = (answer.0.or(after.0), answer.1.or(after.1));
        }
        match answer {
            (Ok(stdout), Ok(stderr)) => Ok(Ran::Finished(Output {
                status,
                stdout,
                stderr,
            })),
            (_, stderr) => Ok(Ran::TimedOut(stderr.unwrap_or_default())),
        }
    }
}

enum Ran {
    Finished(Output),
    TimedOut(Vec<u8>),
}

type Bytes = Result<Vec<u8>, RecvTimeoutError>;

fn within(
    patience: Duration,
    stdout: &Receiver<Vec<u8>>,
    stderr: &Receiver<Vec<u8>>,
) -> (Bytes, Bytes) {
    let until = Instant::now() + patience;
    let left = || until.saturating_duration_since(Instant::now());
    (stdout.recv_timeout(left()), stderr.recv_timeout(left()))
}

fn kill_group(leader: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{leader}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
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
