mod templates;

pub use templates::*;

use crate::config::{self, EmailConfig};
use crate::error::{HeadsupError, Result};
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::time::Duration;

/// Send an email using the configured SMTP settings
pub fn send_email(config: &EmailConfig, content: &EmailContent) -> Result<()> {
    // Parse addresses
    let to_mailbox: Mailbox = config.to.parse()
        .map_err(|e| HeadsupError::Email(format!("Invalid 'to' address: {}", e)))?;
    let from_mailbox: Mailbox = config.from.parse()
        .map_err(|e| HeadsupError::Email(format!("Invalid 'from' address: {}", e)))?;

    // Build message
    let message = Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(&content.subject)
        .header(ContentType::TEXT_PLAIN)
        .body(content.body.clone())
        .map_err(|e| HeadsupError::Email(format!("Failed to build email: {}", e)))?;

    // Build transport
    let mailer = if config.smtp_auth {
        // Authenticated SMTP
        let password_cmd = config.smtp_password_command.as_ref()
            .ok_or_else(|| HeadsupError::ConfigInvalid("smtp_password_command required when smtp_auth is enabled".to_string()))?;
        let username = config.smtp_username.as_ref()
            .ok_or_else(|| HeadsupError::ConfigInvalid("smtp_username required when smtp_auth is enabled".to_string()))?;
        let password = crate::config::get_smtp_password(password_cmd)?;
        let creds = Credentials::new(username.clone(), password);

        SmtpTransport::starttls_relay(&config.smtp_host)
            .map_err(|e| HeadsupError::SmtpConnection(format!("Failed to create SMTP transport: {}", e)))?
            .port(config.smtp_port)
            .credentials(creds)
            .timeout(Some(Duration::from_secs(config.smtp_timeout_seconds)))
            .build()
    } else {
        // Unauthenticated SMTP
        SmtpTransport::builder_dangerous(&config.smtp_host)
            .port(config.smtp_port)
            .timeout(Some(Duration::from_secs(config.smtp_timeout_seconds)))
            .build()
    };

    // Send
    mailer
        .send(&message)
        .map_err(|e| HeadsupError::Email(format!("Failed to send email: {}", e)))?;

    Ok(())
}

/// Send a test email
pub fn send_test_email(config: &EmailConfig) -> Result<()> {
    let content = build_test_email();
    send_email(config, &content)
}

/// Validate email configuration (without sending)
pub fn validate_email_config(config: &EmailConfig) -> Result<()> {
    // Check required fields
    if config.to.is_empty() {
        return Err(HeadsupError::ConfigInvalid("Email 'to' address is required".to_string()));
    }
    if config.from.is_empty() {
        return Err(HeadsupError::ConfigInvalid("Email 'from' address is required".to_string()));
    }
    if config.smtp_host.is_empty() {
        return Err(HeadsupError::ConfigInvalid("SMTP host is required".to_string()));
    }

    // Check auth-related fields only if auth is enabled
    if config.smtp_auth {
        if config.smtp_username.as_ref().map_or(true, |s| s.is_empty()) {
            return Err(HeadsupError::ConfigInvalid("SMTP username is required when smtp_auth is enabled".to_string()));
        }
        if config.smtp_password_command.as_ref().map_or(true, |s| s.is_empty()) {
            return Err(HeadsupError::ConfigInvalid("SMTP password command is required when smtp_auth is enabled".to_string()));
        }
    }

    // Validate email format
    let _: Mailbox = config.to.parse()
        .map_err(|e| HeadsupError::ConfigInvalid(format!("Invalid 'to' address: {}", e)))?;
    let _: Mailbox = config.from.parse()
        .map_err(|e| HeadsupError::ConfigInvalid(format!("Invalid 'from' address: {}", e)))?;

    Ok(())
}
