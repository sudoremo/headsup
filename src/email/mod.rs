pub mod ics;
mod templates;

pub use templates::*;

use crate::config::EmailConfig;
use crate::error::{HeadsupError, Result};
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::time::Duration;

/// Send an email using the configured SMTP settings
pub fn send_email(config: &EmailConfig, content: &EmailContent) -> Result<()> {
    // Get password from command
    let password = crate::config::get_smtp_password(&config.smtp_password_command)?;

    // Parse addresses
    let to_mailbox: Mailbox = config
        .to
        .parse()
        .map_err(|e| HeadsupError::Email(format!("Invalid 'to' address: {}", e)))?;
    let from_mailbox: Mailbox = config
        .from
        .parse()
        .map_err(|e| HeadsupError::Email(format!("Invalid 'from' address: {}", e)))?;

    let builder = Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(&content.subject);

    // Build message: multipart if attachments present, plain text otherwise
    let message = if content.attachments.is_empty() {
        builder
            .header(ContentType::TEXT_PLAIN)
            .body(content.body.clone())
            .map_err(|e| HeadsupError::Email(format!("Failed to build email: {}", e)))?
    } else {
        let text_part = SinglePart::builder()
            .header(ContentType::TEXT_PLAIN)
            .body(content.body.clone());

        let mut multipart = MultiPart::mixed().singlepart(text_part);

        for attachment in &content.attachments {
            let content_type: ContentType = attachment
                .content_type
                .parse()
                .unwrap_or(ContentType::TEXT_PLAIN);
            let ics_attachment = Attachment::new(attachment.filename.clone())
                .body(attachment.data.clone(), content_type);
            multipart = multipart.singlepart(ics_attachment);
        }

        builder
            .multipart(multipart)
            .map_err(|e| HeadsupError::Email(format!("Failed to build email: {}", e)))?
    };

    // Build transport
    let creds = Credentials::new(config.smtp_username.clone(), password);

    let mailer = SmtpTransport::starttls_relay(&config.smtp_host)
        .map_err(|e| {
            HeadsupError::SmtpConnection(format!("Failed to create SMTP transport: {}", e))
        })?
        .port(config.smtp_port)
        .credentials(creds)
        .timeout(Some(Duration::from_secs(config.smtp_timeout_seconds)))
        .build();

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
        return Err(HeadsupError::ConfigInvalid(
            "Email 'to' address is required".to_string(),
        ));
    }
    if config.from.is_empty() {
        return Err(HeadsupError::ConfigInvalid(
            "Email 'from' address is required".to_string(),
        ));
    }
    if config.smtp_host.is_empty() {
        return Err(HeadsupError::ConfigInvalid(
            "SMTP host is required".to_string(),
        ));
    }
    if config.smtp_password_command.is_empty() {
        return Err(HeadsupError::ConfigInvalid(
            "SMTP password command is required".to_string(),
        ));
    }

    // Validate email format
    let _: Mailbox = config
        .to
        .parse()
        .map_err(|e| HeadsupError::ConfigInvalid(format!("Invalid 'to' address: {}", e)))?;
    let _: Mailbox = config
        .from
        .parse()
        .map_err(|e| HeadsupError::ConfigInvalid(format!("Invalid 'from' address: {}", e)))?;

    Ok(())
}
