use std::process::ExitCode;
use thiserror::Error;

/// Exit codes as defined in the specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitStatus {
    Success = 0,
    GeneralError = 1,
    PartialFailure = 2,
    AllSubjectsFailed = 3,
    EmailDeliveryFailed = 4,
    Timeout = 5,
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        ExitCode::from(status as u8)
    }
}

#[derive(Error, Debug)]
pub enum HeadsupError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration file not found at {0}")]
    ConfigNotFound(String),

    #[error("Invalid configuration: {0}")]
    ConfigInvalid(String),

    #[error("State error: {0}")]
    State(String),

    #[error("State file locked by another process")]
    StateLocked,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Claude error: {0}")]
    Claude(String),

    #[error("Claude timeout after {0} seconds")]
    ClaudeTimeout(u64),

    #[error("Claude response parse error: {0}")]
    ClaudeParseError(String),

    #[error("Email error: {0}")]
    Email(String),

    #[error("SMTP connection failed: {0}")]
    SmtpConnection(String),

    #[error("Subject not found: {0}")]
    SubjectNotFound(String),

    #[error("Subject key already exists: {0}")]
    SubjectKeyExists(String),

    #[error("Invalid subject key: {0}")]
    InvalidSubjectKey(String),

    #[error("Password command failed: {0}")]
    PasswordCommand(String),

    #[error("URL validation failed: {0}")]
    InvalidUrl(String),

    #[error("Total run timeout exceeded ({0} seconds)")]
    TotalTimeout(u64),

    #[error("User cancelled operation")]
    UserCancelled,
}

impl HeadsupError {
    /// Convert error to appropriate exit status
    pub fn exit_status(&self) -> ExitStatus {
        match self {
            HeadsupError::Config(_)
            | HeadsupError::ConfigNotFound(_)
            | HeadsupError::ConfigInvalid(_)
            | HeadsupError::Io(_)
            | HeadsupError::TomlParse(_)
            | HeadsupError::TomlSerialize(_)
            | HeadsupError::Json(_)
            | HeadsupError::State(_)
            | HeadsupError::StateLocked
            | HeadsupError::SubjectNotFound(_)
            | HeadsupError::SubjectKeyExists(_)
            | HeadsupError::InvalidSubjectKey(_)
            | HeadsupError::PasswordCommand(_)
            | HeadsupError::InvalidUrl(_)
            | HeadsupError::UserCancelled => ExitStatus::GeneralError,

            HeadsupError::Email(_) | HeadsupError::SmtpConnection(_) => ExitStatus::EmailDeliveryFailed,

            HeadsupError::ClaudeTimeout(_) | HeadsupError::TotalTimeout(_) => ExitStatus::Timeout,

            HeadsupError::Claude(_) | HeadsupError::ClaudeParseError(_) => ExitStatus::GeneralError,
        }
    }
}

pub type Result<T> = std::result::Result<T, HeadsupError>;

// Alias for migration
pub type RadarError = HeadsupError;
