use std::env;
use std::process::ExitCode;

use conveyor::cli::{self, Command};

fn main() -> ExitCode {
    match cli::parse(env::args().skip(1)) {
        Ok(Command::Help) => {
            println!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("conveyor {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Tui | Command::Config) => fail("not implemented yet"),
        Err(err) => fail(&err),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("conveyor: {message}");
    ExitCode::FAILURE
}
