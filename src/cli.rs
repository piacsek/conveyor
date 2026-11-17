pub const USAGE: &str = "usage: conveyor [config | log | --demo]
       conveyor --help | --version";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Tui,
    Demo,
    Config,
    Log,
    Help,
    Version,
}

pub fn parse<I, S>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter().map(|arg| arg.as_ref().to_string());
    let first = args.next();
    if let Some(extra) = args.next() {
        return Err(format!("unknown argument '{extra}'\n{USAGE}"));
    }
    match first.as_deref() {
        None => Ok(Command::Tui),
        Some("config") => Ok(Command::Config),
        Some("--demo") => Ok(Command::Demo),
        Some("log") => Ok(Command::Log),
        Some("--help" | "-h" | "help") => Ok(Command::Help),
        Some("--version" | "-V" | "version") => Ok(Command::Version),
        Some(arg) => Err(format!("unknown argument '{arg}'\n{USAGE}")),
    }
}
