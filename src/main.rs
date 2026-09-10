use std::collections::HashMap;
use std::env;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use conveyor::app::{App, Input, Rows, Stage, run};
use conveyor::cli::{self, Command};
use conveyor::config::{self, Config};
use conveyor::fetch::{fetch_prs, fetch_queue, repos};
use conveyor::github::CliGh;
use conveyor::open::SystemOpener;
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
        Command::Help | Command::Version => Ok(()),
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

fn fail(message: &str) -> ExitCode {
    eprintln!("conveyor: {message}");
    ExitCode::FAILURE
}

type Inputs = mpsc::Sender<io::Result<Input>>;

fn tui(config: Config, config_error: Option<String>) -> io::Result<()> {
    let (tx, rx) = mpsc::channel::<io::Result<Input>>();
    let mut refreshers: HashMap<Stage, mpsc::Sender<()>> = HashMap::new();
    refreshers.insert(Stage::Prs, spawn_prs_fetcher(config.clone(), tx.clone()));
    refreshers.insert(
        Stage::Queue,
        spawn_queue_fetcher(config.clone(), tx.clone()),
    );
    spawn_terminal_events(tx);
    let mut app = App::new(config);
    app.notice = config_error;
    app.utc_offset_secs = chrono::Local::now().offset().local_minus_utc();
    let mut terminal = ratatui::init();
    let result = run(
        &mut terminal,
        &mut app,
        rx.into_iter(),
        &SystemOpener,
        |stage| {
            if let Some(refresh) = refreshers.get(&stage) {
                let _ = refresh.send(());
            }
        },
    );
    ratatui::restore();
    result
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

fn spawn_fetcher(
    stage: Stage,
    interval: Duration,
    tx: Inputs,
    mut fetch: impl FnMut() -> Result<Rows, String> + Send + 'static,
) -> mpsc::Sender<()> {
    let (refresh_tx, refresh_rx) = mpsc::channel::<()>();
    thread::spawn(move || {
        loop {
            if tx.send(Ok(Input::Data(stage, fetch()))).is_err() {
                return;
            }
            match refresh_rx.recv_timeout(interval.max(Duration::from_secs(1))) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    });
    refresh_tx
}

fn spawn_prs_fetcher(config: Config, tx: Inputs) -> mpsc::Sender<()> {
    let interval = Duration::from_secs(config.prs.refresh_secs);
    let gh = CliGh::default();
    spawn_fetcher(Stage::Prs, interval, tx, move || {
        fetch_prs(&gh, &config).map(Rows::Prs)
    })
}

fn spawn_queue_fetcher(config: Config, tx: Inputs) -> mpsc::Sender<()> {
    let gh = CliGh::default();
    let interval = Duration::from_secs(config.repo.first().map_or(30, |repo| repo.refresh_secs));
    let mut repo = None;
    spawn_fetcher(Stage::Queue, interval, tx, move || {
        if repo.is_none() {
            repo = repos(&gh, &config)?.into_iter().next();
        }
        let repo = repo
            .as_ref()
            .ok_or_else(|| "no repository to watch".to_string())?;
        fetch_queue(&gh, repo).map(Rows::Queue)
    })
}
