mod cli;
mod claude;
mod config;
mod email;
mod error;
mod state;
mod ui;

use clap::Parser;
use cli::{Cli, Commands};
use error::{ExitStatus, HeadsupError};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up logging based on verbosity
    setup_logging(cli.verbose, cli.quiet, cli.log_format.as_deref());

    // Run command
    let result = run_command(cli).await;

    match result {
        Ok(status) => status.into(),
        Err(e) => {
            ui::print_error(&e.to_string());
            e.exit_status().into()
        }
    }
}

async fn run_command(cli: Cli) -> Result<ExitStatus, HeadsupError> {
    // Handle global dry-run flag
    let dry_run = cli.dry_run;

    match cli.command {
        Some(Commands::Check {
            subject,
            dry_run: cmd_dry_run,
            force,
            no_notify,
        }) => {
            cli::run_check(subject, dry_run || cmd_dry_run, force, no_notify).await
        }

        Some(Commands::Notify {
            dry_run: cmd_dry_run,
            digest,
        }) => {
            cli::run_notify(dry_run || cmd_dry_run, digest)
        }

        Some(Commands::Subjects { command }) => {
            cli::run_subjects(command).await?;
            Ok(ExitStatus::Success)
        }

        Some(Commands::Config { command }) => {
            cli::run_config(command)?;
            Ok(ExitStatus::Success)
        }

        Some(Commands::State { command }) => {
            cli::run_state(command)?;
            Ok(ExitStatus::Success)
        }

        Some(Commands::History {
            subject,
            limit,
            json,
        }) => {
            cli::run_history(subject, limit, json)?;
            Ok(ExitStatus::Success)
        }

        Some(Commands::Init { force, email }) => {
            cli::run_init(force, email)?;
            Ok(ExitStatus::Success)
        }

        Some(Commands::TestEmail) => {
            run_test_email()?;
            Ok(ExitStatus::Success)
        }

        None => {
            // No command - check if config exists, run init if not
            if !config::config_exists()? {
                ui::print_info("Welcome to Headsup!");
                ui::print_info("Let's set up your configuration.");
                println!();
                cli::run_init(false, None)?;
            } else {
                // Show help
                use clap::CommandFactory;
                let mut cmd = Cli::command();
                cmd.print_help().ok();
            }
            Ok(ExitStatus::Success)
        }
    }
}

fn run_test_email() -> Result<(), HeadsupError> {
    let config = config::load_config()?;

    ui::print_info("Validating email configuration...");
    email::validate_email_config(&config.email)?;

    ui::print_info("Sending test email...");
    email::send_test_email(&config.email)?;

    ui::print_success(&format!("Test email sent to {}", config.email.to));
    Ok(())
}

fn setup_logging(verbose: u8, quiet: bool, format: Option<&str>) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let level = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    match format {
        Some("json") => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json())
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().without_time().with_target(false))
                .init();
        }
    }
}
