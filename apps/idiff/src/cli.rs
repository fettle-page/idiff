//! The command line, as in the README's command list.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "idiff", version, about = "Shows what an installer did to your Windows PC")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run an install command and show what it changed
    Run {
        /// The app's name, if idiff can't tell from the installer
        #[arg(long)]
        app: Option<String>,
        /// Don't ask to open the app once; wait --settle seconds instead
        #[arg(long)]
        yes: bool,
        #[arg(long, value_name = "SECS", default_value_t = 20)]
        settle: u64,
        #[arg(long, value_name = "MINS", default_value_t = 30)]
        timeout: u64,
        /// Don't ask for administrator rights (scheduled tasks and antivirus exclusions stay incomplete)
        #[arg(long)]
        no_elevate: bool,
        #[arg(last = true, required = true, value_name = "COMMAND")]
        command: Vec<String>,
    },
    /// Write a snapshot of everything idiff watches
    Snapshot {
        #[arg(long)]
        no_elevate: bool,
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Compare two snapshot files
    Diff {
        before: PathBuf,
        after: PathBuf,
        #[arg(long)]
        app: Option<String>,
        /// Also list the changes hidden as Windows' own
        #[arg(long)]
        all: bool,
        /// Say why each change was given to the app or to Windows
        #[arg(long)]
        why: bool,
    },
    /// After an uninstall, show what the app left behind
    Leftovers {
        /// A session id, or "last"
        session: String,
        /// Don't ask to uninstall by hand; take the snapshot now
        #[arg(long)]
        yes: bool,
        /// After the uninstall command, wait this long for anything it left running in the background
        #[arg(long, value_name = "SECS", default_value_t = 5)]
        settle: u64,
        /// Give up waiting for the uninstall after this long; the receipt then says so
        #[arg(long, value_name = "MINS", default_value_t = 30)]
        timeout: u64,
        #[arg(long)]
        no_elevate: bool,
        #[arg(last = true, value_name = "UNINSTALL COMMAND")]
        command: Vec<String>,
    },
    /// Show a session's receipt again
    Show {
        /// A session id, or "last"
        session: String,
        /// Also list the changes hidden as Windows' own
        #[arg(long)]
        all: bool,
        /// Say why each change was given to the app or to Windows
        #[arg(long)]
        why: bool,
        #[arg(long)]
        json: bool,
    },
    /// Save a session's receipt as a file, with personal details removed
    Export {
        /// A session id, or "last"
        session: String,
        #[arg(long, value_enum)]
        format: Format,
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: Option<PathBuf>,
        /// Keep user names, PC name and other personal details
        #[arg(long)]
        no_redact: bool,
    },
    /// Show what Windows changes by itself over a few minutes
    Calibrate {
        #[arg(long, value_name = "N")]
        minutes: u64,
        /// Add those changes to your local noise list
        #[arg(long)]
        save: bool,
        /// Don't ask for administrator rights (scheduled tasks and antivirus exclusions stay incomplete)
        #[arg(long)]
        no_elevate: bool,
    },
    /// List the saved sessions
    List,
}

/// Colour for a terminal, unless the user set `NO_COLOR` (an empty one doesn't count).
pub fn color_enabled(terminal: bool, no_color: Option<&std::ffi::OsStr>) -> bool {
    terminal && no_color.is_none_or(|v| v.is_empty())
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Md,
    Html,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn colour_is_for_terminals_without_no_color() {
        use std::ffi::OsStr;
        assert!(color_enabled(true, None));
        assert!(!color_enabled(false, None));
        assert!(!color_enabled(true, Some(OsStr::new("1"))));
        // no-color.org: an empty NO_COLOR doesn't count.
        assert!(color_enabled(true, Some(OsStr::new(""))));
    }

    #[test]
    fn run_takes_everything_after_the_dashes() {
        let cli = Cli::try_parse_from([
            "idiff", "run", "--app", "Discord", "--", "winget", "install", "--id", "x",
        ])
        .unwrap();
        match cli.command {
            Command::Run {
                app, command, settle, ..
            } => {
                assert_eq!(app.as_deref(), Some("Discord"));
                assert_eq!(command, ["winget", "install", "--id", "x"]);
                assert_eq!(settle, 20);
            }
            other => panic!("expected run, got {other:?}"),
        }
        assert!(Cli::try_parse_from(["idiff", "run"]).is_err());
    }
}
