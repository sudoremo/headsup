use crate::claude::{self, ClaudeResponse, QuestionResponse, RecurringResponse, ReleaseResponse};
use crate::config::{self, Backend, Config, Subject, SubjectType};
use crate::email::{self, build_question_email, build_recurring_email, build_release_email};
use crate::error::{ExitStatus, HeadsupError, Result};
use crate::perplexity;
use crate::state::{
    self, Confidence, DatePrecision, HistoryEntry, PendingNotification, QuestionState,
    RecurringState, ReleaseState, ReleaseStatus, State, SubjectState,
};
use crate::ui;
use chrono::Utc;
use futures::future::join_all;
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

    // Get backend-specific settings
    let (total_run_timeout, max_searches, max_failures) = match config.backend {
        Backend::Claude => (
            config.claude.total_run_timeout_seconds,
            config.claude.max_searches_per_run,
            config.claude.max_consecutive_failures,
        ),
        Backend::Perplexity => (
            config.perplexity.total_run_timeout_seconds,
            config.perplexity.max_searches_per_run,
            config.perplexity.max_consecutive_failures,
        ),
    };

    // Start time for total timeout
    let start = Instant::now();
    let total_timeout = if total_run_timeout > 0 {
        Some(Duration::from_secs(total_run_timeout))
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

    // Filter out subjects that have exceeded consecutive failures
    let subjects_to_check: Vec<&Subject> = subjects_to_check
        .into_iter()
        .filter(|subject| {
            if let Some(subject_state) = state.subjects.get(&subject.id) {
                if subject_state.consecutive_failures() >= max_failures {
                    ui::print_warning(&format!(
                        "Skipping '{}' (max consecutive failures reached)",
                        subject.name
                    ));
                    return false;
                }
            }
            true
        })
        .take(max_searches as usize)
        .collect();

    if subjects_to_check.is_empty() {
        ui::print_info("No subjects to check (all skipped)");
        return Ok(ExitStatus::Success);
    }

    ui::print_info(&format!(
        "Checking {} subjects in parallel using {} backend...",
        subjects_to_check.len(),
        match config.backend {
            Backend::Claude => "Claude",
            Backend::Perplexity => "Perplexity",
        }
    ));

    // Clone data for parallel execution
    let config_clone = config.clone();
    let subjects_owned: Vec<Subject> = subjects_to_check.iter().map(|s| (*s).clone()).collect();
    let state_snapshots: Vec<Option<SubjectState>> = subjects_owned
        .iter()
        .map(|s| state.subjects.get(&s.id).cloned())
        .collect();

    // Create futures for parallel execution
    let futures: Vec<_> = subjects_owned
        .into_iter()
        .zip(state_snapshots.into_iter())
        .map(|(subject, state_snapshot)| {
            let cfg = config_clone.clone();
            async move {
                ui::print_info(&format!("  Starting '{}'...", subject.name));
                let result = check_subject_parallel(&cfg, &subject, state_snapshot.as_ref()).await;
                (subject, result)
            }
        })
        .collect();

    // Execute all checks in parallel with timeout
    let parallel_results = if let Some(timeout) = total_timeout {
        let remaining = timeout.saturating_sub(start.elapsed());
        match tokio::time::timeout(remaining, join_all(futures)).await {
            Ok(results) => results,
            Err(_) => {
                ui::print_warning("Total run timeout exceeded during parallel execution");
                Vec::new()
            }
        }
    } else {
        join_all(futures).await
    };

    // Process results sequentially to update state
    let mut results: Vec<CheckResult> = Vec::new();
    for (subject, check_result) in parallel_results {
        match check_result {
            Ok((response, should_notify)) => {
                let result = process_successful_check(
                    &config,
                    &subject,
                    response,
                    should_notify,
                    &mut state,
                    dry_run,
                    no_notify,
                );
                results.push(result);
            }
            Err(e) => {
                let result = process_failed_check(&config, &subject, e, &mut state, dry_run, max_failures);
                results.push(result);
            }
        }
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

/// Check a single subject using the configured backend (for parallel execution)
async fn check_subject_parallel(
    config: &Config,
    subject: &Subject,
    state: Option<&SubjectState>,
) -> Result<(ClaudeResponse, bool)> {
    let response = match config.backend {
        Backend::Claude => claude::check_subject(&config.claude, subject, state).await?,
        Backend::Perplexity => perplexity::check_subject(&config.perplexity, subject, state).await?,
    };

    let should_notify = match &response {
        ClaudeResponse::Release(r) => r.should_notify,
        ClaudeResponse::Question(r) => r.should_notify,
        ClaudeResponse::Recurring(r) => r.should_notify,
        ClaudeResponse::SubjectIdentification(_) => false,
    };

    Ok((response, should_notify))
}

/// Process a successful check result
fn process_successful_check(
    config: &Config,
    subject: &Subject,
    response: ClaudeResponse,
    should_notify: bool,
    state: &mut State,
    dry_run: bool,
    no_notify: bool,
) -> CheckResult {
    let mut result = CheckResult {
        subject_id: subject.id,
        subject_name: subject.name.clone(),
        success: true,
        notified: false,
        error: None,
    };

    // Clone state for notification
    let previous_state = state.subjects.get(&subject.id).cloned();

    // Process response based on type
    let notify_flag = match &response {
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

    if notify_flag && !no_notify && !dry_run {
        match send_notification(config, subject, &response, previous_state.as_ref()) {
            Ok(()) => {
                result.notified = true;
                ui::print_success(&format!("  Notified about '{}'", subject.name));
            }
            Err(e) => {
                ui::print_error(&format!("  Failed to send notification: {}", e));
            }
        }
    } else if notify_flag && no_notify && !dry_run {
        add_pending_notification(subject, &response, state);
        ui::print_info(&format!("  Added '{}' to pending notifications", subject.name));
    } else if notify_flag {
        ui::print_info(&format!("  Would notify about '{}' (dry run)", subject.name));
    } else {
        ui::print_info(&format!("  '{}' - no changes", subject.name));
    }

    result
}

/// Process a failed check result
fn process_failed_check(
    config: &Config,
    subject: &Subject,
    error: HeadsupError,
    state: &mut State,
    dry_run: bool,
    max_failures: u32,
) -> CheckResult {
    let mut result = CheckResult {
        subject_id: subject.id,
        subject_name: subject.name.clone(),
        success: false,
        notified: false,
        error: Some(error.to_string()),
    };

    ui::print_error(&format!("  '{}' error: {}", subject.name, error));

    // Increment failure count
    if !dry_run {
        let failure_reason = match &error {
            HeadsupError::ClaudeTimeout(_) | HeadsupError::PerplexityTimeout(_) => "timeout",
            HeadsupError::ClaudeParseError(_) => "parse_error",
            _ => "api_error",
        };

        if let Some(subject_state) = state.subjects.get_mut(&subject.id) {
            subject_state.increment_failure(failure_reason);
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

