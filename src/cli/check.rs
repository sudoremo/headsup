use crate::claude::{self, ClaudeResponse, QuestionResponse, RecurringResponse, ReleaseResponse};
use crate::config::{self, Config, Subject, SubjectType};
use crate::email::{self, build_question_email, build_recurring_email, build_release_email, build_subject_disabled_email};
use crate::error::{ExitStatus, HeadsupError, Result};
use crate::state::{
    self, Confidence, DatePrecision, HistoryEntry, PendingNotification, QuestionState,
    RecurringState, ReleaseState, ReleaseStatus, State, SubjectState,
};
use crate::ui;
use chrono::Utc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Result of checking a single subject
pub struct CheckResult {
    pub subject_id: Uuid,
    pub subject_name: String,
    pub success: bool,
    pub notified: bool,
    pub error: Option<String>,
}

/// Run the check command
pub async fn run_check(
    subject_key: Option<String>,
    dry_run: bool,
    force: bool,
    no_notify: bool,
) -> Result<ExitStatus> {
    let config = config::load_config()?;
    let (mut state, lock) = state::load_state()?;

    // Start time for total timeout
    let start = Instant::now();
    let total_timeout = if config.claude.total_run_timeout_seconds > 0 {
        Some(Duration::from_secs(config.claude.total_run_timeout_seconds))
    } else {
        None
    };

    // Determine which subjects to check
    let subjects_to_check: Vec<&Subject> = if let Some(ref key) = subject_key {
        let subject = config
            .find_subject(key)
            .ok_or_else(|| HeadsupError::SubjectNotFound(key.clone()))?;
        vec![subject]
    } else {
        config.subjects.iter().filter(|s| s.enabled).collect()
    };

    if subjects_to_check.is_empty() {
        ui::print_info("No subjects to check");
        return Ok(ExitStatus::Success);
    }

    let mut results: Vec<CheckResult> = Vec::new();
    let mut search_count = 0;

    for subject in subjects_to_check {
        // Check total timeout
        if let Some(timeout) = total_timeout {
            if start.elapsed() > timeout {
                ui::print_warning("Total run timeout exceeded");
                break;
            }
        }

        // Check search limit
        if search_count >= config.claude.max_searches_per_run {
            ui::print_warning("Max searches per run reached");
            break;
        }

        // Check consecutive failures
        if let Some(subject_state) = state.subjects.get(&subject.id) {
            if subject_state.consecutive_failures() >= config.claude.max_consecutive_failures {
                ui::print_warning(&format!(
                    "Skipping '{}' (max consecutive failures reached)",
                    subject.name
                ));
                continue;
            }
        }

        ui::print_info(&format!("Checking '{}'...", subject.name));
        search_count += 1;

        let result = check_single_subject(&config, subject, &mut state, dry_run, no_notify).await;
        results.push(result);
    }

    // Update state
    state.last_run = Some(Utc::now());
    if !dry_run {
        state::save_state(&state, &lock)?;
    }

    // Determine exit status
    let success_count = results.iter().filter(|r| r.success).count();
    let failure_count = results.iter().filter(|r| !r.success).count();
    let notify_count = results.iter().filter(|r| r.notified).count();

    // Print summary
    ui::print_blank();
    ui::print_info(&format!(
        "Checked {} subjects: {} succeeded, {} failed, {} notifications",
        results.len(),
        success_count,
        failure_count,
        notify_count
    ));

    if failure_count == 0 {
        Ok(ExitStatus::Success)
    } else if success_count == 0 {
        Ok(ExitStatus::AllSubjectsFailed)
    } else {
        Ok(ExitStatus::PartialFailure)
    }
}

async fn check_single_subject(
    config: &Config,
    subject: &Subject,
    state: &mut State,
    dry_run: bool,
    no_notify: bool,
) -> CheckResult {
    let mut result = CheckResult {
        subject_id: subject.id,
        subject_name: subject.name.clone(),
        success: false,
        notified: false,
        error: None,
    };

    // Clone current state for Claude call (avoids borrow conflict)
    let current_state_for_claude = state.subjects.get(&subject.id).cloned();

    // Call Claude
    match claude::check_subject(&config.claude, subject, current_state_for_claude.as_ref()).await {
        Ok(response) => {
            result.success = true;

            // Clone state again before mutation for notification
            let previous_state = state.subjects.get(&subject.id).cloned();

            // Process response based on type
            let should_notify = match &response {
                ClaudeResponse::Release(r) => {
                    process_release_response(config, subject, r, state, dry_run)
                }
                ClaudeResponse::Question(r) => {
                    process_question_response(config, subject, r, state, dry_run)
                }
                ClaudeResponse::Recurring(r) => {
                    process_recurring_response(config, subject, r, state, dry_run)
                }
                ClaudeResponse::SubjectIdentification(_) => false,
            };

            if should_notify && !no_notify && !dry_run {
                // Send notification
                match send_notification(config, subject, &response, previous_state.as_ref()) {
                    Ok(()) => {
                        result.notified = true;
                        ui::print_success(&format!("  Notified about '{}'", subject.name));
                    }
                    Err(e) => {
                        ui::print_error(&format!("  Failed to send notification: {}", e));
                    }
                }
            } else if should_notify && no_notify && !dry_run {
                // Add to pending notifications
                add_pending_notification(subject, &response, state);
                ui::print_info(&format!("  Added '{}' to pending notifications", subject.name));
            } else if should_notify {
                ui::print_info(&format!("  Would notify about '{}' (dry run)", subject.name));
            }
        }
        Err(e) => {
            result.error = Some(e.to_string());
            ui::print_error(&format!("  Error: {}", e));

            // Increment failure count
            if !dry_run {
                let failure_reason = match &e {
                    HeadsupError::ClaudeTimeout(_) => "timeout",
                    HeadsupError::ClaudeParseError(_) => "parse_error",
                    _ => "claude_error",
                };

                if let Some(subject_state) = state.subjects.get_mut(&subject.id) {
                    subject_state.increment_failure(failure_reason);

                    // Check if we should disable the subject
                    if subject_state.consecutive_failures() >= config.claude.max_consecutive_failures {
                        // Auto-disable subject and notify user
                        disable_subject_and_notify(config, subject);
                    }
                }
            }
        }
    }

    result
}

fn process_release_response(
    config: &Config,
    subject: &Subject,
    response: &ReleaseResponse,
    state: &mut State,
    dry_run: bool,
) -> bool {
    let release_state = state.get_or_create_release(subject.id);
    let should_notify = response.should_notify;

    if !dry_run {
        // Update state
        release_state.last_checked = Some(Utc::now());
        release_state.known_release_date = response.found_release_date.clone();
        release_state.release_date_precision = response.release_date_precision;
        release_state.confidence = response.confidence;
        release_state.status = response.status;
        release_state.reset_failures();

        if should_notify {
            release_state.last_notified = Some(Utc::now());
        }

        // Add history entry
        let entry = HistoryEntry {
            timestamp: Utc::now(),
            event: "check".to_string(),
            details: serde_json::json!({
                "found_release_date": response.found_release_date,
                "precision": response.release_date_precision.to_string(),
                "confidence": response.confidence.to_string(),
                "status": response.status.to_string(),
                "should_notify": should_notify,
            }),
            source_url: response.source_url.clone(),
            raw_response: Some(serde_json::to_string(response).unwrap_or_default()),
        };
        state.add_history(subject.id, entry, config.settings.max_history_entries);
    }

    should_notify
}

fn process_question_response(
    config: &Config,
    subject: &Subject,
    response: &QuestionResponse,
    state: &mut State,
    dry_run: bool,
) -> bool {
    let question_state = state.get_or_create_question(subject.id);
    let should_notify = response.should_notify;

    if !dry_run {
        question_state.last_checked = Some(Utc::now());
        question_state.current_answer = response.found_answer.clone();
        question_state.confidence = response.confidence;
        question_state.is_definitive = response.is_definitive;
        question_state.reset_failures();

        if should_notify {
            question_state.last_notified = Some(Utc::now());
        }

        let entry = HistoryEntry {
            timestamp: Utc::now(),
            event: "check".to_string(),
            details: serde_json::json!({
                "found_answer": response.found_answer,
                "confidence": response.confidence.to_string(),
                "is_definitive": response.is_definitive,
                "should_notify": should_notify,
            }),
            source_url: response.source_url.clone(),
            raw_response: Some(serde_json::to_string(response).unwrap_or_default()),
        };
        state.add_history(subject.id, entry, config.settings.max_history_entries);
    }

    should_notify
}

fn process_recurring_response(
    config: &Config,
    subject: &Subject,
    response: &RecurringResponse,
    state: &mut State,
    dry_run: bool,
) -> bool {
    let recurring_state = state.get_or_create_recurring(subject.id);
    let should_notify = response.should_notify;

    if !dry_run {
        recurring_state.last_checked = Some(Utc::now());
        recurring_state.next_occurrence_date = response.next_occurrence_date.clone();
        recurring_state.next_occurrence_name = response.next_occurrence_name.clone();
        recurring_state.date_precision = response.date_precision;
        recurring_state.confidence = response.confidence;
        recurring_state.reset_failures();

        if should_notify {
            recurring_state.last_notified = Some(Utc::now());
        }

        let entry = HistoryEntry {
            timestamp: Utc::now(),
            event: "check".to_string(),
            details: serde_json::json!({
                "next_occurrence_date": response.next_occurrence_date,
                "next_occurrence_name": response.next_occurrence_name,
                "date_precision": response.date_precision.to_string(),
                "confidence": response.confidence.to_string(),
                "should_notify": should_notify,
            }),
            source_url: response.source_url.clone(),
            raw_response: Some(serde_json::to_string(response).unwrap_or_default()),
        };
        state.add_history(subject.id, entry, config.settings.max_history_entries);
    }

    should_notify
}

fn send_notification(
    config: &Config,
    subject: &Subject,
    response: &ClaudeResponse,
    previous_state: Option<&SubjectState>,
) -> Result<()> {
    let content = match response {
        ClaudeResponse::Release(r) => {
            let prev = previous_state.and_then(|s| match s {
                SubjectState::Release(rs) => Some(rs),
                _ => None,
            });
            build_release_email(subject, r, prev)
        }
        ClaudeResponse::Question(r) => {
            let prev = previous_state.and_then(|s| match s {
                SubjectState::Question(qs) => Some(qs),
                _ => None,
            });
            build_question_email(subject, r, prev)
        }
        ClaudeResponse::Recurring(r) => {
            let prev = previous_state.and_then(|s| match s {
                SubjectState::Recurring(rs) => Some(rs),
                _ => None,
            });
            build_recurring_email(subject, r, prev)
        }
        ClaudeResponse::SubjectIdentification(_) => {
            return Ok(()); // Should never happen
        }
    };

    email::send_email(&config.email, &content)
}

fn add_pending_notification(subject: &Subject, response: &ClaudeResponse, state: &mut State) {
    let (event_type, summary, source_url, payload) = match response {
        ClaudeResponse::Release(r) => (
            "release_update".to_string(),
            r.summary.clone(),
            r.source_url.clone(),
            serde_json::to_value(r).unwrap_or_default(),
        ),
        ClaudeResponse::Question(r) => (
            "question_update".to_string(),
            r.summary.clone(),
            r.source_url.clone(),
            serde_json::to_value(r).unwrap_or_default(),
        ),
        ClaudeResponse::Recurring(r) => (
            "recurring_update".to_string(),
            r.summary.clone(),
            r.source_url.clone(),
            serde_json::to_value(r).unwrap_or_default(),
        ),
        ClaudeResponse::SubjectIdentification(_) => return,
    };

    state.add_pending_notification(PendingNotification {
        subject_id: subject.id,
        event_type,
        created_at: Utc::now(),
        summary,
        source_url,
        payload,
    });
}

fn disable_subject_and_notify(config: &Config, subject: &Subject) {
    // Try to disable the subject in config
    if let Ok(mut cfg) = config::load_config() {
        if let Some(s) = cfg.find_subject_mut(&subject.key) {
            s.enabled = false;
            let _ = config::save_config(&cfg);
        }
    }

    // Send notification email
    let content = build_subject_disabled_email(subject, config.claude.max_consecutive_failures);
    let _ = email::send_email(&config.email, &content);

    ui::print_warning(&format!(
        "Subject '{}' auto-disabled after {} failures",
        subject.name, config.claude.max_consecutive_failures
    ));
}
