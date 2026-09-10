use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use conveyor::cli::{self, Command};
use conveyor::config;

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
    let config = match config::load(&config_path()) {
        Ok(config) => config,
        Err(err) => return fail(&err.to_string()),
    };
    match command {
        Command::Config => {
            print!("{}", config.to_toml());
            ExitCode::SUCCESS
        }
        Command::Tui => fail("not implemented yet"),
        Command::Help | Command::Version => ExitCode::SUCCESS,
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
