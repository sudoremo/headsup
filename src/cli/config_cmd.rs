use crate::cli::ConfigCommands;
use crate::config::{self, Config};
use crate::email;
use crate::error::{HeadsupError, Result};
use crate::ui;
use std::path::PathBuf;
use std::process::Command;

/// Run config subcommands
pub fn run_config(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show => show_config(),
        ConfigCommands::Edit => edit_config(),
        ConfigCommands::Validate => validate_config(),
        ConfigCommands::Path => print_path(),
        ConfigCommands::Export => export_config(),
        ConfigCommands::Import { file } => import_config(file),
    }
}

fn show_config() -> Result<()> {
    let config = config::load_config()?;
    let redacted = config::redact_config(&config);
    let content = toml::to_string_pretty(&redacted)
        .map_err(|e| HeadsupError::Config(format!("Failed to serialize config: {}", e)))?;
    println!("{}", content);
    Ok(())
}

fn edit_config() -> Result<()> {
    let path = config::config_path()?;

    if !path.exists() {
        return Err(HeadsupError::ConfigNotFound(path.display().to_string()));
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| HeadsupError::Config(format!("Failed to launch editor '{}': {}", editor, e)))?;

    if !status.success() {
        return Err(HeadsupError::Config(format!("Editor exited with status {}", status)));
    }

    // Validate the config after editing
    match config::load_config() {
        Ok(config) => {
            if let Err(errors) = config.validate() {
                ui::print_warning("Config has validation errors:");
                for error in errors {
                    ui::print_error(&format!("  {}", error));
                }
            } else {
                ui::print_success("Config is valid");
            }
        }
        Err(e) => {
            ui::print_error(&format!("Config has syntax errors: {}", e));
        }
    }

    Ok(())
}

fn validate_config() -> Result<()> {
    let config = config::load_config()?;

    // Validate structure
    match config.validate() {
        Ok(warnings) => {
            for warning in warnings {
                ui::print_warning(&warning);
            }
            ui::print_success("Config is valid");
        }
        Err(errors) => {
            for error in errors {
                ui::print_error(&error);
            }
            return Err(HeadsupError::ConfigInvalid("Config validation failed".to_string()));
        }
    }

    // Validate email config
    email::validate_email_config(&config.email)?;
    ui::print_success("Email configuration is valid");

    Ok(())
}

fn print_path() -> Result<()> {
    let path = config::config_path()?;
    println!("{}", path.display());
    Ok(())
}

fn export_config() -> Result<()> {
    let config = config::load_config()?;
    let redacted = config::redact_config(&config);
    let content = toml::to_string_pretty(&redacted)
        .map_err(|e| HeadsupError::Config(format!("Failed to serialize config: {}", e)))?;
    print!("{}", content);
    Ok(())
}

fn import_config(file: PathBuf) -> Result<()> {
    // Load existing config
    let mut config = config::load_config().unwrap_or_else(|_| Config::default_with_email("user@example.com"));

    // Load import file
    let import_config = config::load_config_from(&file)?;

    // Merge subjects (add new ones, skip duplicates by key)
    let existing_keys: std::collections::HashSet<String> = config
        .subjects
        .iter()
        .map(|s| s.key.to_lowercase())
        .collect();

    let mut added = 0;
    let mut skipped = 0;

    for subject in import_config.subjects {
        if existing_keys.contains(&subject.key.to_lowercase()) {
            skipped += 1;
        } else {
            config.subjects.push(subject);
            added += 1;
        }
    }

    // Save merged config
    config::save_config(&config)?;

    ui::print_success(&format!(
        "Imported {} subjects ({} skipped as duplicates)",
        added, skipped
    ));

    Ok(())
}
