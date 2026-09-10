pub const USAGE: &str = "usage: conveyor [config]
       conveyor --help | --version";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Tui,
    Config,
    Help,
    Version,
}

pub fn parse<I, S>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter().map(|arg| arg.as_ref().to_string());
    match args.next().as_deref() {
        None => Ok(Command::Tui),
        Some("config") => Ok(Command::Config),
        Some("--help" | "-h" | "help") => Ok(Command::Help),
        Some("--version" | "-V" | "version") => Ok(Command::Version),
        Some(arg) => Err(format!("unknown argument '{arg}'\n{USAGE}")),
    }
}
