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
use conveyor::fetch::fetch_prs;
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

fn tui(config: Config, config_error: Option<String>) -> io::Result<()> {
    let (inputs, refresh) = spawn_sources(config.clone());
    let mut app = App::new(config);
    app.notice = config_error;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, inputs, &SystemOpener, |stage| {
        let _ = refresh.send(stage);
    });
    ratatui::restore();
    result
}

fn spawn_sources(config: Config) -> (impl Iterator<Item = io::Result<Input>>, mpsc::Sender<Stage>) {
    let (tx, rx) = mpsc::channel::<io::Result<Input>>();
    let (refresh_tx, refresh_rx) = mpsc::channel::<Stage>();
    spawn_prs_fetcher(config, tx.clone(), refresh_rx);
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
    (rx.into_iter(), refresh_tx)
}

fn spawn_prs_fetcher(
    config: Config,
    tx: mpsc::Sender<io::Result<Input>>,
    refresh: mpsc::Receiver<Stage>,
) {
    thread::spawn(move || {
        let gh = CliGh::default();
        let interval = Duration::from_secs(config.prs.refresh_secs.max(1));
        loop {
            let data = fetch_prs(&gh, &config).map(Rows::Prs);
            if tx.send(Ok(Input::Data(Stage::Prs, data))).is_err() {
                return;
            }
            match refresh.recv_timeout(interval) {
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    });
}
