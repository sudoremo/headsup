use crate::config::{self, Config};
use crate::email::{self, build_digest_email, EmailContent};
use crate::error::{ExitStatus, Result};
use crate::state::{self, PendingNotification};
use crate::ui;

/// Run the notify command
pub fn run_notify(dry_run: bool, digest: bool) -> Result<ExitStatus> {
    let config = config::load_config()?;
    let (mut state, lock) = state::load_state()?;

    if state.pending_notifications.is_empty() {
        ui::print_info("No pending notifications");
        return Ok(ExitStatus::Success);
    }

    let notifications = state.clear_pending_notifications();
    let count = notifications.len();

    ui::print_info(&format!("Sending {} pending notifications...", count));

    // Determine if we should use digest mode
    let use_digest = digest || config.email.digest_mode;

    if dry_run {
        if use_digest {
            ui::print_info("Would send digest email with:");
            for notif in &notifications {
                let subject_name = config.subjects.iter()
                    .find(|s| s.id == notif.subject_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("Unknown");
                ui::print_info(&format!("  - {} ({})", subject_name, notif.event_type));
            }
        } else {
            for notif in &notifications {
                let subject_name = config.subjects.iter()
                    .find(|s| s.id == notif.subject_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("Unknown");
                ui::print_info(&format!("Would send: {} - {}", subject_name, notif.event_type));
            }
        }
        return Ok(ExitStatus::Success);
    }

    let result = if use_digest {
        send_digest(&config, &notifications)
    } else {
        send_individual(&config, &notifications)
    };

    match result {
        Ok(sent) => {
            // Save state (notifications cleared)
            state::save_state(&state, &lock)?;
            ui::print_success(&format!("Sent {} notifications", sent));
            Ok(ExitStatus::Success)
        }
        Err(e) => {
            // Put notifications back on failure
            for notif in notifications {
                state.add_pending_notification(notif);
            }
            state::save_state(&state, &lock)?;
            ui::print_error(&format!("Failed to send notifications: {}", e));
            Ok(ExitStatus::EmailDeliveryFailed)
        }
    }
}

fn send_digest(config: &Config, notifications: &[PendingNotification]) -> Result<usize> {
    let content = build_digest_email(notifications, &config.subjects);
    email::send_email(&config.email, &content)?;
    Ok(1)
}

fn send_individual(config: &Config, notifications: &[PendingNotification]) -> Result<usize> {
    let mut sent = 0;

    for notif in notifications {
        let subject = config.subjects.iter()
            .find(|s| s.id == notif.subject_id);

        let subject_name = subject
            .map(|s| s.name.as_str())
            .unwrap_or("Unknown");

        let content = EmailContent {
            subject: format!("[Headsup] {} - {}", subject_name, notif.event_type),
            body: format!(
                "{}\n\nSource: {}\n\nThis is an automated message from Headsup.",
                notif.summary,
                notif.source_url.as_deref().unwrap_or("N/A")
            ),
        };

        email::send_email(&config.email, &content)?;
        sent += 1;
    }

    Ok(sent)
}
