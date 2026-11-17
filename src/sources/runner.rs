use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(10);
const DRAIN: Duration = Duration::from_millis(500);

pub enum Ran {
    Finished(Output),
    TimedOut(Vec<u8>),
}

pub fn run(command: &mut Command, deadline: Option<Duration>) -> io::Result<Ran> {
    let mut child = {
        let _alone = super::spawning();
        command
            .process_group(0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if deadline.is_some_and(|deadline| started.elapsed() >= deadline) {
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
