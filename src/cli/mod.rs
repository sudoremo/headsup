mod check;
mod config_cmd;
mod history;
mod init;
mod notify;
mod state_cmd;
mod subjects;

pub use check::run_check;
pub use config_cmd::run_config;
pub use history::run_history;
pub use init::run_init;
pub use notify::run_notify;
pub use state_cmd::run_state;
pub use subjects::run_subjects;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "headsup")]
#[command(author, version, about = "Monitor subjects for release dates and answers")]
#[command(propagate_version = true)]
pub struct Cli {
    /// Increase log verbosity (can repeat: -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long)]
    pub quiet: bool,

    /// Output format: text (default) or json
    #[arg(long, value_name = "FORMAT")]
    pub log_format: Option<String>,

    /// Use alternate config file
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Check but don't send emails or update state
    #[arg(long)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a check for all subjects, or a specific one
    Check {
        /// Check only this subject (by key or UUID)
        subject: Option<String>,

        /// Don't send emails or update state
        #[arg(long)]
        dry_run: bool,

        /// Check even if recently checked
        #[arg(long)]
        force: bool,

        /// Only check and update state, don't send emails
        #[arg(long)]
        no_notify: bool,
    },

    /// Send pending notifications
    Notify {
        /// Show what would be sent without sending
        #[arg(long)]
        dry_run: bool,

        /// Force digest mode for this run
        #[arg(long)]
        digest: bool,
    },

    /// Manage monitored subjects
    Subjects {
        #[command(subcommand)]
        command: SubjectsCommands,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Manage state
    State {
        #[command(subcommand)]
        command: StateCommands,
    },

    /// View notification history
    History {
        /// Show history for specific subject only
        subject: Option<String>,

        /// Show only last N entries
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Initialize config and state files
    Init {
        /// Overwrite existing files
        #[arg(long)]
        force: bool,

        /// Set email address during init
        #[arg(long)]
        email: Option<String>,
    },

    /// Send a test email to verify SMTP configuration
    TestEmail,
}

#[derive(Subcommand)]
pub enum SubjectsCommands {
    /// List all subjects with status
    List,

    /// Add a new subject (interactive, AI-assisted)
    Add,

    /// Remove a subject
    Remove {
        /// Subject key or UUID
        key: String,
    },

    /// Edit a subject (interactive)
    Edit {
        /// Subject key or UUID
        key: String,
    },

    /// Enable a disabled subject
    Enable {
        /// Subject key or UUID
        key: String,
    },

    /// Disable a subject without removing
    Disable {
        /// Subject key or UUID
        key: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show config (secrets redacted)
    Show,

    /// Open config in $EDITOR
    Edit,

    /// Validate config file
    Validate,

    /// Print config file path
    Path,

    /// Export config to stdout (secrets redacted)
    Export,

    /// Import config from file (merges subjects)
    Import {
        /// File to import
        file: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum StateCommands {
    /// Show current state
    Show,

    /// Remove orphaned entries (subjects not in config)
    Prune,

    /// Reset state for a subject (or all if no key)
    Reset {
        /// Subject key or UUID
        key: Option<String>,
    },

    /// Print state file path
    Path,
}
