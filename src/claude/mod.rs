mod process;
mod prompt;
mod response;

pub use process::execute_claude;
pub use prompt::*;
pub use response::*;

use crate::config::{ClaudeConfig, Subject, SubjectType};
use crate::error::Result;
use crate::state::SubjectState;

/// Check a subject and return the response
pub async fn check_subject(
    config: &ClaudeConfig,
    subject: &Subject,
    state: Option<&SubjectState>,
) -> Result<ClaudeResponse> {
    match subject.subject_type {
        SubjectType::Release => {
            let release_state = state.and_then(|s| match s {
                SubjectState::Release(rs) => Some(rs),
                _ => None,
            });
            let prompt = build_release_prompt(subject, release_state);
            let raw = execute_claude(config, &prompt).await?;
            let response = parse_release_response(&raw)?;
            Ok(ClaudeResponse::Release(response))
        }
        SubjectType::Question => {
            let question_state = state.and_then(|s| match s {
                SubjectState::Question(qs) => Some(qs),
                _ => None,
            });
            let prompt = build_question_prompt(subject, question_state);
            let raw = execute_claude(config, &prompt).await?;
            let response = parse_question_response(&raw)?;
            Ok(ClaudeResponse::Question(response))
        }
        SubjectType::Recurring => {
            let recurring_state = state.and_then(|s| match s {
                SubjectState::Recurring(rs) => Some(rs),
                _ => None,
            });
            let prompt = build_recurring_prompt(subject, recurring_state);
            let raw = execute_claude(config, &prompt).await?;
            let response = parse_recurring_response(&raw)?;
            Ok(ClaudeResponse::Recurring(response))
        }
    }
}

/// Identify subjects based on user input
pub async fn identify_subjects(
    config: &ClaudeConfig,
    user_input: &str,
) -> Result<SubjectIdentificationResponse> {
    let prompt = build_subject_identification_prompt(user_input);
    let raw = execute_claude(config, &prompt).await?;
    parse_subject_identification_response(&raw)
}
