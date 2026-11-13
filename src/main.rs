use std::collections::HashMap;
use std::env;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use conveyor::app::{
    App, CheckoutTarget, Input, MergeTarget, Request, Rows, Stage, StatusTarget, UpdateTarget, run,
};
use conveyor::cli::{self, Command};
use conveyor::config::{self, Config, expand_home};
use conveyor::log::{self, Log};
use conveyor::open::SystemOpener;
use conveyor::refresh;
use conveyor::sources::fetch::{
    BuildsSource, DeploySource, fetch_jobs, fetch_log, fetch_prs, fetch_queue, repos, rerun_failed,
};
use conveyor::sources::github::{CliGh, Github};
use conveyor::sources::kube::CliKubectl;
use conveyor::tmux;
use ratatui::crossterm::event::{self, Event};

fn main() -> ExitCode {
    let command = match cli::parse(env::args().skip(1)) {
        Ok(Command::Help) => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        Ok(Command::Version) => {
            println!("conveyor {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Ok(Command::Log) => {
            return match log_path() {
                Some(path) => {
                    println!("{}", path.display());
                    ExitCode::SUCCESS
                }
                None => fail("no log: HOME is not an absolute path"),
            };
        }
        Ok(command) => command,
        Err(err) => return fail(&err),
    };
    let (config, config_error) = match config::load(&config_path()) {
        Ok(config) => (config, None),
        Err(err) => (Config::default(), Some(err.to_string())),
    };
    if let Some(err) = &config_error {
        eprintln!("conveyor: {err}");
    }
    let result = match command {
        Command::Config if config_error.is_some() => return ExitCode::FAILURE,
        Command::Config => {
            print!("{}", config.to_toml());
            Ok(())
        }
        Command::Tui => tui(config, config_error),
        Command::Help | Command::Version | Command::Log => Ok(()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => fail(&err.to_string()),
    }
}

fn config_path() -> PathBuf {
    config::path(
        env::var_os("CONVEYOR_CONFIG").map(PathBuf::from),
        env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        &env::var_os("HOME").map(PathBuf::from).unwrap_or_default(),
    )
}

fn log_path() -> Option<PathBuf> {
    log::path(
        env::var_os("CONVEYOR_LOG").map(PathBuf::from),
        env::var_os("XDG_STATE_HOME").map(PathBuf::from),
        &env::var_os("HOME").map(PathBuf::from).unwrap_or_default(),
        &log::program_name(env::args_os().next()),
    )
}

fn fail(message: &str) -> ExitCode {
    eprintln!("conveyor: {message}");
    ExitCode::FAILURE
}

type Inputs = mpsc::Sender<io::Result<Input>>;

fn tui(config: Config, config_error: Option<String>) -> io::Result<()> {
    let log = log_path().map_or_else(Log::default, |path| Log::open(&path));
    log.line(&format!(
        "conveyor {} started; config {}",
        env!("CARGO_PKG_VERSION"),
        config_path().display()
    ));
    if let Some(err) = &config_error {
        log.line(&format!("config error: {err}"));
    }
    let (tx, rx) = mpsc::channel::<io::Result<Input>>();
    let mut refreshers: HashMap<Stage, mpsc::Sender<()>> = HashMap::new();
    refreshers.insert(
        Stage::Prs,
        spawn_prs_fetcher(&log, config.clone(), tx.clone()),
    );
    refreshers.insert(
        Stage::Queue,
        spawn_queue_fetcher(&log, config.clone(), tx.clone()),
    );
    refreshers.insert(
        Stage::Builds,
        spawn_builds_fetcher(&log, config.clone(), tx.clone()),
    );
    if let Some(refresh) = spawn_deploy_fetcher(&log, config.clone(), tx.clone()) {
        refreshers.insert(Stage::Deployed, refresh);
    }
    let jobs = spawn_jobs_fetcher(&log, config.clone(), tx.clone());
    let logs = spawn_log_fetcher(&log, config.clone(), tx.clone());
    let reruns = spawn_rerun_worker(&log, tx.clone());
    let merges = spawn_merger(&log, config.clone(), tx.clone());
    let updates = spawn_branch_updater(&log, tx.clone());
    let statuses = spawn_status_changer(&log, tx.clone());
    let checkouts = spawn_checkout_worker(&log, tx.clone());
    spawn_terminal_events(tx);
    let links_fix = tmux::ask(&log).map_or_else(Vec::new, |answer| tmux::links_fix(&answer));
    for line in &links_fix {
        log.line(line.trim_start());
    }
    let mut app = App::new(config);
    app.notice =
        config_error.or_else(|| (!links_fix.is_empty()).then(|| tmux::LINKS_NOTICE.to_string()));
    app.links_fix = links_fix;
    app.utc_offset_secs = chrono::Local::now().offset().local_minus_utc();
    let mut terminal = ratatui::init();
    let result = run(
        &mut terminal,
        &mut app,
        rx.into_iter(),
        &SystemOpener::default(),
        |request| match request {
            Request::Refresh(stage) => {
                if let Some(refresh) = refreshers.get(&stage) {
                    let _ = refresh.send(());
                }
            }
            Request::Jobs { run_id } => {
                let _ = jobs.send(run_id);
            }
            Request::Log { run_id, job_id } => {
                let _ = logs.send((run_id, job_id));
            }
            Request::Merge(target) => {
                let _ = merges.send(target);
            }
            Request::UpdateBranch(target) => {
                let _ = updates.send(target);
            }
            Request::ChangeStatus(target) => {
                let _ = statuses.send(target);
            }
            Request::Checkout(target) => {
                let _ = checkouts.send(target);
            }
            Request::Rerun { repo, run_ids } => {
                let _ = reruns.send((repo, run_ids));
            }
        },
    );
    ratatui::restore();
    log.line("conveyor quit");
    result
}

fn spawn_jobs_fetcher(log: &Log, config: Config, tx: Inputs) -> mpsc::Sender<u64> {
    let (jobs_tx, jobs_rx) = mpsc::channel::<u64>();
    let gh = CliGh::default().with_log(log.clone());
    thread::spawn(move || {
        let mut repo: Option<String> = None;
        while let Ok(run_id) = jobs_rx.recv() {
            if repo.is_none() {
                repo = repos(&gh, &config)
                    .ok()
                    .and_then(|repos| repos.into_iter().next())
                    .map(|repo| repo.name);
            }
            let result = match &repo {
                Some(name) => fetch_jobs(&gh, name, run_id),
                None => Err("no repository to watch".to_string()),
            };
            if tx.send(Ok(Input::Jobs(run_id, result))).is_err() {
                return;
            }
        }
    });
    jobs_tx
}

fn spawn_log_fetcher(log: &Log, config: Config, tx: Inputs) -> mpsc::Sender<(u64, u64)> {
    let (log_tx, log_rx) = mpsc::channel::<(u64, u64)>();
    let gh = CliGh::default().with_log(log.clone());
    thread::spawn(move || {
        let mut repo: Option<String> = None;
        while let Ok((run_id, job_id)) = log_rx.recv() {
            if repo.is_none() {
                repo = repos(&gh, &config)
                    .ok()
                    .and_then(|repos| repos.into_iter().next())
                    .map(|repo| repo.name);
            }
            let result = match &repo {
                Some(name) => fetch_log(&gh, name, run_id, job_id),
                None => Err("no repository to watch".to_string()),
            };
            if tx.send(Ok(Input::Log(job_id, result))).is_err() {
                return;
            }
        }
    });
    log_tx
}

fn spawn_rerun_worker(log: &Log, tx: Inputs) -> mpsc::Sender<(String, Vec<u64>)> {
    let (rerun_tx, rerun_rx) = mpsc::channel::<(String, Vec<u64>)>();
    let gh = CliGh::default().with_log(log.clone());
    thread::spawn(move || {
        while let Ok((repo, run_ids)) = rerun_rx.recv() {
            let result = rerun_failed(&gh, &repo, &run_ids);
            if tx.send(Ok(Input::Rerun(result))).is_err() {
                return;
            }
        }
    });
    rerun_tx
}

fn spawn_branch_updater(log: &Log, tx: Inputs) -> mpsc::Sender<UpdateTarget> {
    let (update_tx, update_rx) = mpsc::channel::<UpdateTarget>();
    let gh = CliGh::default().with_log(log.clone());
    thread::spawn(move || {
        while let Ok(target) = update_rx.recv() {
            let result = gh
                .update_branch(&target.repo, target.number, target.method)
                .map_err(|err| err.to_string());
            if tx
                .send(Ok(Input::BranchUpdated(target.number, result)))
                .is_err()
            {
                return;
            }
        }
    });
    update_tx
}

fn spawn_status_changer(log: &Log, tx: Inputs) -> mpsc::Sender<StatusTarget> {
    let (status_tx, status_rx) = mpsc::channel::<StatusTarget>();
    let gh = CliGh::default().with_log(log.clone());
    thread::spawn(move || {
        while let Ok(target) = status_rx.recv() {
            let result = gh
                .change_status(&target.repo, target.number, &target.change)
                .map_err(|err| err.to_string());
            let landed = Input::StatusChanged(target.number, target.change, result);
            if tx.send(Ok(landed)).is_err() {
                return;
            }
        }
    });
    status_tx
}

fn spawn_checkout_worker(log: &Log, tx: Inputs) -> mpsc::Sender<CheckoutTarget> {
    let (checkout_tx, checkout_rx) = mpsc::channel::<CheckoutTarget>();
    let gh = CliGh::default().with_log(log.clone());
    let home = env::var_os("HOME").map(PathBuf::from);
    thread::spawn(move || {
        while let Ok(target) = checkout_rx.recv() {
            let dir = target
                .dir
                .as_deref()
                .map(|dir| expand_home(dir, home.as_deref()));
            let result = gh
                .checkout(&target.repo, target.number, dir.as_deref())
                .map_err(|err| err.to_string());
            if tx.send(Ok(Input::CheckedOut(target, result))).is_err() {
                return;
            }
        }
    });
    checkout_tx
}

fn spawn_merger(log: &Log, config: Config, tx: Inputs) -> mpsc::Sender<MergeTarget> {
    let (merge_tx, merge_rx) = mpsc::channel::<MergeTarget>();
    let gh = CliGh::default().with_log(log.clone());
    thread::spawn(move || {
        while let Ok(target) = merge_rx.recv() {
            let result = gh
                .merge(
                    &target.repo,
                    target.number,
                    config.prs.merge_method,
                    &target.head_oid,
                )
                .map_err(|err| err.to_string());
            if tx.send(Ok(Input::Merged(target.number, result))).is_err() {
                return;
            }
        }
    });
    merge_tx
}

fn spawn_terminal_events(tx: Inputs) {
    thread::spawn(move || {
        loop {
            let input = match event::poll(Duration::from_millis(250)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key)) if key.is_press() => Input::Key(key),
                    Ok(_) => continue,
                    Err(err) => {
                        let _ = tx.send(Err(err));
                        return;
                    }
                },
                Ok(false) => Input::Tick(SystemTime::now()),
                Err(err) => {
                    let _ = tx.send(Err(err));
                    return;
                }
            };
            if tx.send(Ok(input)).is_err() {
                return;
            }
        }
    });
}

fn spawn_prs_fetcher(log: &Log, config: Config, tx: Inputs) -> mpsc::Sender<()> {
    let interval = Duration::from_secs(config.prs.refresh_secs);
    let gh = CliGh::default().with_log(log.clone());
    refresh::spawn(Stage::Prs, interval, tx, move || {
        fetch_prs(&gh, &config).map(Rows::Prs)
    })
}

fn spawn_queue_fetcher(log: &Log, config: Config, tx: Inputs) -> mpsc::Sender<()> {
    let gh = CliGh::default().with_log(log.clone());
    let interval = Duration::from_secs(config.repo.first().map_or(30, |repo| repo.refresh_secs));
    let mut repo = None;
    refresh::spawn(Stage::Queue, interval, tx, move || {
        if repo.is_none() {
            repo = repos(&gh, &config)?.into_iter().next();
        }
        let repo = repo
            .as_ref()
            .ok_or_else(|| "no repository to watch".to_string())?;
        fetch_queue(&gh, repo).map(Rows::Queue)
    })
}

fn spawn_builds_fetcher(log: &Log, config: Config, tx: Inputs) -> mpsc::Sender<()> {
    let gh = CliGh::default().with_log(log.clone());
    let interval = Duration::from_secs(config.repo.first().map_or(30, |repo| repo.refresh_secs));
    let mut source: Option<BuildsSource> = None;
    refresh::spawn(Stage::Builds, interval, tx, move || {
        if source.is_none() {
            let repo = repos(&gh, &config)?
                .into_iter()
                .next()
                .ok_or_else(|| "no repository to watch".to_string())?;
            source = Some(BuildsSource::new(repo));
        }
        let source = source
            .as_mut()
            .ok_or_else(|| "no repository to watch".to_string())?;
        let builds = source.fetch(&gh, SystemTime::now())?;
        Ok(Rows::Builds(source.into_builds(builds)))
    })
}

fn spawn_deploy_fetcher(log: &Log, config: Config, tx: Inputs) -> Option<mpsc::Sender<()>> {
    let repo = config.repo.iter().find(|repo| !repo.deploy.is_empty())?;
    let deploy = repo.deploy.first()?.clone();
    let interval = Duration::from_secs(deploy.refresh_secs);
    let mut source = DeploySource::new(repo.name.clone(), deploy);
    let gh = CliGh::default().with_log(log.clone());
    let kube = CliKubectl::default().with_log(log.clone());
    Some(refresh::spawn(Stage::Deployed, interval, tx, move || {
        Ok(Rows::Deployed(source.fetch(&gh, &kube, SystemTime::now())))
    }))
}
