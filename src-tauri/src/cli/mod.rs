//! The `hamster` command line. Without a subcommand it starts the app (or activates the running one); a
//! subcommand asks the running app over the socket and prints its answer (`reply`). A new subcommand adds a
//! `Command` variant, its `IpcMessage`, its arm in `remote::handle` and, when it prints something, in
//! `reply::render`.

pub mod reply;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

use crate::ipc::IpcMessage;
use crate::profiles::cli::ProfileRequest;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

const EXAMPLES: &str = "Examples:
  hamster                  start Hamster or bring it to the front
  hamster status --json    whether it runs, its version and its window
  hamster quit             quit the running app
  hamster profile switch Alex   open the profile named Alex";

#[derive(Parser, Debug)]
#[command(
    name = "hamster",
    version,
    about = "Private macOS app for ANV documents, bank statements and subscriptions",
    disable_help_subcommand = true,
    after_help = EXAMPLES
)]
struct Cli {
    /// Start without showing the window; a running instance stays as it is
    #[arg(long)]
    hidden: bool,
    /// Log level for this run
    #[arg(long, value_enum)]
    log_level: Option<LogLevel>,
    /// Print replies as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Whether the app runs, its version and its window
    Status,
    /// Show the window without bringing the app to the front; starts the app when it isn't running
    Show,
    /// Show the window and bring the app to the front; starts the app when it isn't running
    Focus,
    /// Hide the window; the app keeps running
    Hide,
    /// Quit the running app
    Quit,
    /// Run a command by its id, as its menu item or shortcut would
    Run { id: String },
    /// List every command with its id and shortcut
    Commands,
    /// List the profiles (the active one marked with *), or switch, add or rename one; works without the app
    Profile {
        #[command(subcommand)]
        command: Option<ProfileCommand>,
    },
    /// Debug build only: tools for its demo data
    #[cfg(debug_assertions)]
    #[command(subcommand)]
    Dev(DevCommand),
}

#[cfg(debug_assertions)]
#[derive(Subcommand, Debug)]
enum DevCommand {
    /// Quit the debug app and remove its data, so the next launch seeds the demo data again
    Seed {
        /// Confirms removing the debug app's data
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ProfileCommand {
    /// Every profile; * marks the one the app opens
    List,
    /// Open the profile called NAME; a running app switches to it in place
    Switch { name: String },
    /// Add an empty profile called NAME
    Add {
        name: String,
        /// Also switch to it
        #[arg(long)]
        switch: bool,
    },
    /// Rename the profile called NAME to NEW_NAME
    Rename { name: String, new_name: String },
}

impl From<ProfileCommand> for ProfileRequest {
    fn from(command: ProfileCommand) -> Self {
        match command {
            ProfileCommand::List => ProfileRequest::List,
            ProfileCommand::Switch { name } => ProfileRequest::Switch { name },
            ProfileCommand::Add { name, switch } => ProfileRequest::Add { name, switch },
            ProfileCommand::Rename { name, new_name } => {
                ProfileRequest::Rename { name, to: new_name }
            }
        }
    }
}

/// A message for the running app and how to print its reply.
#[derive(Debug, PartialEq)]
pub struct CliRequest {
    pub message: IpcMessage,
    pub json: bool,
}

#[derive(Debug, PartialEq)]
pub enum CliOutcome {
    /// Start the app, or hand the launch to the running one.
    Run {
        hidden: bool,
        log_level: Option<LogLevel>,
    },
    /// `--help`, `--version` or a usage error: print `message` and exit with `code` (0 or 2).
    Exit { message: String, code: i32 },
    /// A subcommand the running app answers; it never starts the app.
    Ask(CliRequest),
    /// A subcommand the running app answers, or the app starts with it (`show`, `focus`).
    Remote(CliRequest),
    /// `profile …`, routed in `lib.rs` `answer_profile`.
    Profile { request: ProfileRequest, json: bool },
    /// `dev seed --yes`: quit the running debug app, then remove its data.
    #[cfg(debug_assertions)]
    ResetDemo,
}

/// Parses the full argv (including the program name).
pub fn parse<I, S>(args: I) -> CliOutcome
where
    I: IntoIterator<Item = S>,
    S: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            return CliOutcome::Exit {
                message: e.render().to_string(),
                code: e.exit_code(),
            };
        }
    };
    let Some(command) = cli.command else {
        return CliOutcome::Run {
            hidden: cli.hidden,
            log_level: cli.log_level,
        };
    };
    #[cfg(debug_assertions)]
    if let Command::Dev(DevCommand::Seed { yes }) = command {
        return if yes {
            CliOutcome::ResetDemo
        } else {
            CliOutcome::Exit {
                message: "This removes the debug app's data; run again with --yes\n".into(),
                code: 2,
            }
        };
    }
    if matches!(command, Command::Commands) {
        return CliOutcome::Exit {
            message: reply::commands_text(&crate::commands::list(), cli.json),
            code: 0,
        };
    }
    if let Command::Profile { command } = command {
        return CliOutcome::Profile {
            request: command.map_or(ProfileRequest::List, ProfileRequest::from),
            json: cli.json,
        };
    }
    let starts_app = matches!(command, Command::Show | Command::Focus);
    let request = CliRequest {
        message: message(command),
        json: cli.json,
    };
    if starts_app {
        CliOutcome::Remote(request)
    } else {
        CliOutcome::Ask(request)
    }
}

fn message(command: Command) -> IpcMessage {
    match command {
        Command::Status => IpcMessage::Status,
        Command::Show => IpcMessage::Show { focus: false },
        Command::Focus => IpcMessage::Activate { hidden: false },
        Command::Hide => IpcMessage::Hide,
        Command::Quit => IpcMessage::Quit,
        Command::Run { id } => IpcMessage::Run { id },
        Command::Commands | Command::Profile { .. } => unreachable!("answered earlier in parse"),
        #[cfg(debug_assertions)]
        Command::Dev(_) => unreachable!("answered without the app"),
    }
}

#[cfg(test)]
mod tests;
