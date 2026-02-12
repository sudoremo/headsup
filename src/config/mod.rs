mod types;

pub use types::*;

use crate::error::{HeadsupError, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the XDG-compliant config directory
pub fn config_dir() -> Result<PathBuf> {
    ProjectDirs::from("", "", "headsup")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or_else(|| HeadsupError::Config("Could not determine config directory".to_string()))
}

/// Get the XDG-compliant data directory
pub fn data_dir() -> Result<PathBuf> {
    ProjectDirs::from("", "", "headsup")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or_else(|| HeadsupError::Config("Could not determine data directory".to_string()))
}

/// Get the config file path
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Get the state file path
pub fn state_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("state.json"))
}

/// Check if config file exists
pub fn config_exists() -> Result<bool> {
    Ok(config_path()?.exists())
}

/// Load config from file
pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Err(HeadsupError::ConfigNotFound(path.display().to_string()));
    }

    let content = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

/// Load config from a specific path
pub fn load_config_from(path: &PathBuf) -> Result<Config> {
    if !path.exists() {
        return Err(HeadsupError::ConfigNotFound(path.display().to_string()));
    }

    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

/// Save config to file
pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(config)?;
    fs::write(&path, content)?;
    Ok(())
}

/// Save config to a specific path
pub fn save_config_to(config: &Config, path: &PathBuf) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
}

/// Execute the password command and return the password
pub fn get_smtp_password(command: &str) -> Result<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .output()
    } else {
        Command::new("sh")
            .args(["-c", command])
            .output()
    };

    match output {
        Ok(output) => {
            if output.status.success() {
                let password = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
                if password.is_empty() {
                    Err(HeadsupError::PasswordCommand("Password command returned empty output".to_string()))
                } else {
                    Ok(password)
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(HeadsupError::PasswordCommand(format!(
                    "Password command failed: {}",
                    stderr.trim()
                )))
            }
        }
        Err(e) => Err(HeadsupError::PasswordCommand(format!(
            "Failed to execute password command: {}",
            e
        ))),
    }
}

/// Create default config with placeholder values
pub fn create_default_config(email: &str) -> Result<()> {
    let config = Config::default_with_email(email);
    save_config(&config)
}

/// Redact sensitive information from config for display
pub fn redact_config(config: &Config) -> Config {
    let mut redacted = config.clone();
    if redacted.email.smtp_password_command.is_some() {
        redacted.email.smtp_password_command = Some("[REDACTED]".to_string());
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        assert_eq!(Subject::generate_key("GTA 6"), "gta-6");
        assert_eq!(Subject::generate_key("The Last of Us"), "the-last-of-us");
        assert_eq!(Subject::generate_key("Rust 2024 Edition"), "rust-2024-edition");
    }

    #[test]
    fn test_subject_validation() {
        let mut subject = Subject {
            id: uuid::Uuid::new_v4(),
            key: "test".to_string(),
            name: "Test".to_string(),
            subject_type: SubjectType::Release,
            category: Some(Category::Game),
            question: None,
            event_name: None,
            search_terms: vec!["test".to_string()],
            notes: None,
            enabled: true,
        };
        assert!(subject.validate().is_ok());

        // Missing category for release
        subject.category = None;
        assert!(subject.validate().is_err());

        // Question type without question
        subject.subject_type = SubjectType::Question;
        subject.category = None;
        assert!(subject.validate().is_err());

        subject.question = Some("Who is the next Bond?".to_string());
        assert!(subject.validate().is_ok());
    }
}
