use std::collections::HashMap;
use std::env;
use std::io;
use std::panic;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use conveyor::app::{
    App, CheckoutTarget, Input, MergeTarget, Request, Rows, Stage, StatusTarget, UpdateTarget, run,
};
use conveyor::cli::{self, Command};
use conveyor::config::{self, Config, Repo, expand_home};
use conveyor::crash;
use conveyor::events::{self, Crossterm, SystemClock};
use conveyor::log::{self, Log};
use conveyor::open::{DemoOpener, Opener, SystemOpener};
use conveyor::refresh;
use conveyor::sources::demo::{self, Demo};
use conveyor::sources::fetch::{
    BuildsSource, DeploySource, fetch_jobs, fetch_log, fetch_prs, fetch_queue, repos, rerun_failed,
};
use conveyor::sources::github::{CliGh, Github};
use conveyor::sources::kube::{CliKubectl, Kube};
use conveyor::worker;

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
        Command::Tui => {
            let log = log_path().map_or_else(Log::default, |path| Log::open(&path));
            let gh_log = log.clone();
            let kube_log = log.clone();
            tui(
                config,
                config_error,
                log,
                move || CliGh::default().with_log(gh_log.clone()),
                move || CliKubectl::default().with_log(kube_log.clone()),
                SystemOpener::default(),
                false,
            )
        }
        Command::Demo => tui(
            demo::config(config),
            config_error,
            Log::default(),
            Demo::default,
            Demo::default,
            DemoOpener::default(),
            true,
        ),
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

fn tui<G, K>(
    config: Config,
    config_error: Option<String>,
    log: Log,
    gh: impl Fn() -> G,
    kube: impl Fn() -> K,
    opener: impl Opener,
    demo: bool,
) -> io::Result<()>
where
    G: Github + Send + 'static,
    K: Kube + Sync + Send + 'static,
{
    log.line(&format!(
        "conveyor {} started; config {}",
        env!("CARGO_PKG_VERSION"),
        config_path().display()
    ));
    if let Some(err) = &config_error {
        log.line(&format!("config error: {err}"));
    }
    let mut terminal = ratatui::init();
    panic::set_hook(crash::hook(log.clone(), panic::take_hook()));
    let (tx, rx) = mpsc::channel::<io::Result<Input>>();
    let mut refreshers: HashMap<Stage, mpsc::Sender<usize>> = HashMap::new();
    refreshers.insert(
        Stage::Prs,
        spawn_prs_fetcher(gh(), config.clone(), tx.clone()),
    );
    refreshers.insert(
        Stage::Queue,
        spawn_queue_fetcher(gh(), config.clone(), tx.clone()),
    );
    refreshers.insert(
        Stage::Builds,
        spawn_builds_fetchers(&gh, &config, tx.clone()),
    );
    if let Some(refresh) = spawn_deploy_fetcher(gh(), kube(), config.clone(), tx.clone()) {
        refreshers.insert(Stage::Deployed, refresh);
    }
    let jobs_gh = gh();
    let jobs = worker::spawn(tx.clone(), move |(repo, run_id): (String, u64)| {
        Input::Jobs(run_id, fetch_jobs(&jobs_gh, &repo, run_id))
    });
    let logs_gh = gh();
    let logs = worker::spawn(
        tx.clone(),
        move |(repo, run_id, job_id): (String, u64, u64)| {
            Input::Log(job_id, fetch_log(&logs_gh, &repo, run_id, job_id))
        },
    );
    let reruns_gh = gh();
    let reruns = worker::spawn(tx.clone(), move |(repo, run_ids): (String, Vec<u64>)| {
        Input::Rerun(rerun_failed(&reruns_gh, &repo, &run_ids))
    });
    let merges_gh = gh();
    let merge_method = config.prs.merge_method;
    let merges = worker::spawn(tx.clone(), move |target: MergeTarget| {
        let result = merges_gh.merge(&target.repo, target.number, merge_method, &target.head_oid);
        Input::Merged(target, result)
    });
    let updates_gh = gh();
    let updates = worker::spawn(tx.clone(), move |target: UpdateTarget| {
        let result = updates_gh.update_branch(&target.repo, target.number, target.method);
        Input::BranchUpdated(target, result)
    });
    let statuses_gh = gh();
    let statuses = worker::spawn(tx.clone(), move |target: StatusTarget| {
        let result = statuses_gh.change_status(&target.repo, target.number, &target.change);
        Input::StatusChanged(target, result)
    });
    let checkouts_gh = gh();
    let home = env::var_os("HOME").map(PathBuf::from);
    let checkouts = worker::spawn(tx.clone(), move |target: CheckoutTarget| {
        let dir = target
            .dir
            .as_deref()
            .map(|dir| expand_home(dir, home.as_deref()));
        let result = checkouts_gh.checkout(&target.repo, target.number, dir.as_deref());
        Input::CheckedOut(target, result)
    });
    spawn_terminal_events(tx);
    let mut app = App::new(config);
    app.notice = config_error;
    app.demo = demo;
    app.utc_offset_secs = chrono::Local::now().offset().local_minus_utc();
    let result = run(
        &mut terminal,
        &mut app,
        rx.into_iter(),
        &opener,
        |request| match request {
            Request::Refresh(stage, tab) => {
                if let Some(refresh) = refreshers.get(&stage) {
                    let _ = refresh.send(tab);
                }
            }
            Request::Jobs { repo, run_id } => {
                let _ = jobs.send((repo, run_id));
            }
            Request::Log {
                repo,
                run_id,
                job_id,
            } => {
                let _ = logs.send((repo, run_id, job_id));
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

fn spawn_terminal_events(tx: Inputs) {
    thread::Builder::new()
        .name("terminal events".to_string())
        .spawn(move || events::forward(&mut Crossterm, &SystemClock, &tx))
        .expect("the terminal events thread starts");
}

fn spawn_prs_fetcher(
    gh: impl Github + Send + 'static,
    config: Config,
    tx: Inputs,
) -> mpsc::Sender<usize> {
    let interval = Duration::from_secs(config.prs.refresh_secs);
    let tabs = if config.prs.background_tabs {
        config.prs.tabs().len()
    } else {
        1
    };
    refresh::spawn(Stage::Prs, interval, tabs, tx, move |tab| {
        fetch_prs(&gh, &config, tab).map(Rows::Prs)
    })
}

fn spawn_queue_fetcher(
    gh: impl Github + Send + 'static,
    config: Config,
    tx: Inputs,
) -> mpsc::Sender<usize> {
    let interval = Duration::from_secs(config.repo.first().map_or(30, |repo| repo.refresh_secs));
    let mut watched: Option<Vec<Repo>> = None;
    refresh::spawn(Stage::Queue, interval, 1, tx, move |tab| {
        if watched.is_none() {
            watched = Some(repos(&gh, &config)?);
        }
        let repo = watched
            .iter()
            .flatten()
            .nth(tab)
            .ok_or_else(|| "no repository to watch".to_string())?;
        fetch_queue(&gh, repo).map(Rows::Queue)
    })
}

fn spawn_builds_fetchers<G: Github + Send + 'static>(
    gh: impl Fn() -> G,
    config: &Config,
    tx: Inputs,
) -> mpsc::Sender<usize> {
    let intervals = (0..config.repo.len().max(1))
        .map(|tab| Duration::from_secs(config.repo.get(tab).map_or(30, |repo| repo.refresh_secs)))
        .collect();
    refresh::spawn_each(Stage::Builds, intervals, tx, |tab| {
        let gh = gh();
        let config = config.clone();
        let mut source: Option<BuildsSource> = None;
        move || {
            if source.is_none() {
                let repo = repos(&gh, &config)?
                    .into_iter()
                    .nth(tab)
                    .ok_or_else(|| "no repository to watch".to_string())?;
                source = Some(BuildsSource::new(repo));
            }
            let source = source
                .as_mut()
                .ok_or_else(|| "no repository to watch".to_string())?;
            let builds = source.fetch(&gh, SystemTime::now())?;
            Ok(Rows::Builds(source.into_builds(builds)))
        }
    })
}

fn spawn_deploy_fetcher(
    gh: impl Github + Send + 'static,
    kube: impl Kube + Sync + Send + 'static,
    config: Config,
    tx: Inputs,
) -> Option<mpsc::Sender<usize>> {
    let repo = config.repo.iter().find(|repo| !repo.deploy.is_empty())?;
    let deploy = repo.deploy.first()?.clone();
    let interval = Duration::from_secs(deploy.refresh_secs);
    let mut source = DeploySource::new(repo.name.clone(), deploy);
    Some(refresh::spawn(
        Stage::Deployed,
        interval,
        1,
        tx,
        move |_| Ok(Rows::Deployed(source.fetch(&gh, &kube, SystemTime::now()))),
    ))
}
