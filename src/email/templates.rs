use crate::claude::{QuestionResponse, RecurringResponse, ReleaseResponse};
use crate::config::Subject;
use crate::state::{PendingNotification, QuestionState, RecurringState, ReleaseState};

const SEPARATOR: &str = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
const FOOTER: &str = "This is an automated message from Headsup.";

/// Email content (subject line and body)
pub struct EmailContent {
    pub subject: String,
    pub body: String,
}

/// Build email content for a release notification
pub fn build_release_email(
    subject: &Subject,
    response: &ReleaseResponse,
    previous_state: Option<&ReleaseState>,
) -> EmailContent {
    let event_type = determine_release_event_type(response, previous_state);
    let email_subject = format!("[Headsup] {} - {}", subject.name, event_type);

    let previous_info = if let Some(state) = previous_state {
        if let Some(ref date) = state.known_release_date {
            format!("Previous Status:\n  Release date: {} ({})", date, state.confidence)
        } else {
            "Previous Status:\n  No release date was previously known.".to_string()
        }
    } else {
        "Previous Status:\n  No release date was previously known.".to_string()
    };

    let source_info = response.source_url.as_ref()
        .map(|url| format!("Source:\n  {}", url))
        .unwrap_or_else(|| "Source:\n  No source URL available".to_string());

    let body = format!(
        r#"{separator}

{name} - {event_type}

New Information:
  {summary}

{previous_info}

{source_info}

Confidence: {confidence}

{separator}

{footer}"#,
        separator = SEPARATOR,
        name = subject.name,
        event_type = event_type,
        summary = response.summary,
        previous_info = previous_info,
        source_info = source_info,
        confidence = response.confidence,
        footer = FOOTER
    );

    EmailContent {
        subject: email_subject,
        body,
    }
}

/// Build email content for a question notification
pub fn build_question_email(
    subject: &Subject,
    response: &QuestionResponse,
    previous_state: Option<&QuestionState>,
) -> EmailContent {
    let event_type = determine_question_event_type(response, previous_state);
    let email_subject = format!("[Headsup] {} - {}", subject.name, event_type);

    let question = subject.question.as_ref()
        .map(|q| q.as_str())
        .unwrap_or("Unknown question");

    let previous_info = if let Some(state) = previous_state {
        if let Some(ref answer) = state.current_answer {
            format!("Previous Status:\n  {} ({})", answer, state.confidence)
        } else {
            "Previous Status:\n  No answer was previously known.".to_string()
        }
    } else {
        "Previous Status:\n  No answer was previously known.".to_string()
    };

    let answer_info = response.found_answer.as_ref()
        .map(|a| format!("Answer:\n  {}", a))
        .unwrap_or_else(|| "Answer:\n  No answer found.".to_string());

    let source_info = response.source_url.as_ref()
        .map(|url| format!("Source:\n  {}", url))
        .unwrap_or_else(|| "Source:\n  No source URL available".to_string());

    let body = format!(
        r#"{separator}

{name} - {event_type}

Question:
  {question}

{answer_info}

{previous_info}

{source_info}

Confidence: {confidence}

{separator}

{footer}"#,
        separator = SEPARATOR,
        name = subject.name,
        event_type = event_type,
        question = question,
        answer_info = answer_info,
        previous_info = previous_info,
        source_info = source_info,
        confidence = response.confidence,
        footer = FOOTER
    );

    EmailContent {
        subject: email_subject,
        body,
    }
}

/// Build email content for a recurring event notification
pub fn build_recurring_email(
    subject: &Subject,
    response: &RecurringResponse,
    previous_state: Option<&RecurringState>,
) -> EmailContent {
    let event_type = determine_recurring_event_type(response, previous_state);
    let email_subject = format!("[Headsup] {} - {}", subject.name, event_type);

    let default_event_name = subject.event_name.clone().unwrap_or_default();
    let event_name = response.next_occurrence_name.as_ref()
        .unwrap_or(&default_event_name);

    let date_info = response.next_occurrence_date.as_ref()
        .map(|d| format!("Date: {}", d))
        .unwrap_or_else(|| "Date: Unknown".to_string());

    let previous_info = if let Some(state) = previous_state {
        if let Some(ref date) = state.last_occurrence_date {
            format!("Previous Event:\n  {}", date)
        } else {
            "Previous Event:\n  No previous event recorded.".to_string()
        }
    } else {
        "Previous Event:\n  No previous event recorded.".to_string()
    };

    let source_info = response.source_url.as_ref()
        .map(|url| format!("Source:\n  {}", url))
        .unwrap_or_else(|| "Source:\n  No source URL available".to_string());

    let body = format!(
        r#"{separator}

{subject_name} - {event_type}

Event: {event_name}
{date_info}

Details:
  {summary}

{previous_info}

{source_info}

{separator}

{footer}"#,
        separator = SEPARATOR,
        subject_name = subject.name,
        event_type = event_type,
        event_name = event_name,
        date_info = date_info,
        summary = response.summary,
        previous_info = previous_info,
        source_info = source_info,
        footer = FOOTER
    );

    EmailContent {
        subject: email_subject,
        body,
    }
}

/// Build a digest email combining multiple notifications
pub fn build_digest_email(notifications: &[PendingNotification], subjects: &[Subject]) -> EmailContent {
    let email_subject = format!("[Headsup] {} Updates", notifications.len());

    let mut items = Vec::new();
    for notif in notifications {
        let subject_name = subjects.iter()
            .find(|s| s.id == notif.subject_id)
            .map(|s| s.name.as_str())
            .unwrap_or("Unknown");

        items.push(format!(
            "- {} ({})\n  {}",
            subject_name,
            notif.event_type,
            notif.summary
        ));
    }

    let body = format!(
        r#"{separator}

Headsup - {count} Updates

{items}

{separator}

{footer}"#,
        separator = SEPARATOR,
        count = notifications.len(),
        items = items.join("\n\n"),
        footer = FOOTER
    );

    EmailContent {
        subject: email_subject,
        body,
    }
}

/// Build a test email
pub fn build_test_email() -> EmailContent {
    EmailContent {
        subject: "[Headsup] Test Email".to_string(),
        body: format!(
            r#"{separator}

Headsup - Test Email

This is a test email to verify your SMTP configuration is working correctly.

If you're reading this, your email settings are configured properly!

{separator}

{footer}"#,
            separator = SEPARATOR,
            footer = FOOTER
        ),
    }
}

/// Build email for subject auto-disabled notification
pub fn build_subject_disabled_email(subject: &Subject, failures: u32) -> EmailContent {
    EmailContent {
        subject: format!("[Headsup] Subject '{}' Disabled", subject.name),
        body: format!(
            r#"{separator}

Subject Auto-Disabled

The subject "{name}" has been automatically disabled after {failures} consecutive failures.

To re-enable this subject, run:
  headsup subjects enable {key}

{separator}

{footer}"#,
            separator = SEPARATOR,
            name = subject.name,
            failures = failures,
            key = subject.key,
            footer = FOOTER
        ),
    }
}

fn determine_release_event_type(response: &ReleaseResponse, previous: Option<&ReleaseState>) -> &'static str {
    match previous {
        None => {
            if response.found_release_date.is_some() {
                "Release Date Announced"
            } else {
                "Status Update"
            }
        }
        Some(state) => {
            if state.known_release_date.is_none() && response.found_release_date.is_some() {
                "Release Date Announced"
            } else if state.known_release_date != response.found_release_date {
                "Release Date Changed"
            } else if response.release_date_precision.is_more_precise_than(&state.release_date_precision) {
                "Release Date Refined"
            } else if response.confidence.is_higher_than(&state.confidence) {
                "Confidence Upgraded"
            } else {
                "Status Update"
            }
        }
    }
}

fn determine_question_event_type(response: &QuestionResponse, previous: Option<&QuestionState>) -> &'static str {
    match previous {
        None => {
            if response.found_answer.is_some() {
                "Answer Found"
            } else {
                "Status Update"
            }
        }
        Some(state) => {
            if state.current_answer.is_none() && response.found_answer.is_some() {
                "Answer Found"
            } else if state.current_answer != response.found_answer {
                "Answer Changed"
            } else if response.confidence.is_higher_than(&state.confidence) {
                "Confidence Upgraded"
            } else if response.is_definitive && !state.is_definitive {
                "Answer Confirmed"
            } else {
                "Status Update"
            }
        }
    }
}

fn determine_recurring_event_type(response: &RecurringResponse, previous: Option<&RecurringState>) -> &'static str {
    match previous {
        None => {
            if response.next_occurrence_date.is_some() {
                "Next Event Announced"
            } else {
                "Status Update"
            }
        }
        Some(state) => {
            if state.next_occurrence_date.is_none() && response.next_occurrence_date.is_some() {
                "Next Event Announced"
            } else if state.next_occurrence_date != response.next_occurrence_date {
                "Event Date Changed"
            } else {
                "Status Update"
            }
        }
    }
}
