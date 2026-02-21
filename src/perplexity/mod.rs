mod process;

pub use process::execute_perplexity;

use crate::config::{PerplexityConfig, Subject, SubjectType};
use crate::claude::{
    build_release_prompt, build_question_prompt, build_recurring_prompt,
    parse_release_response, parse_question_response, parse_recurring_response,
    ClaudeResponse,
};
use crate::error::Result;
use crate::state::SubjectState;

/// Check a subject using Perplexity API and return the response
pub async fn check_subject(
    config: &PerplexityConfig,
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
            let raw = execute_perplexity(config, &prompt).await?;
            let response = parse_release_response(&raw)?;
            Ok(ClaudeResponse::Release(response))
        }
        SubjectType::Question => {
            let question_state = state.and_then(|s| match s {
                SubjectState::Question(qs) => Some(qs),
                _ => None,
            });
            let prompt = build_question_prompt(subject, question_state);
            let raw = execute_perplexity(config, &prompt).await?;
            let response = parse_question_response(&raw)?;
            Ok(ClaudeResponse::Question(response))
        }
        SubjectType::Recurring => {
            let recurring_state = state.and_then(|s| match s {
                SubjectState::Recurring(rs) => Some(rs),
                _ => None,
            });
            let prompt = build_recurring_prompt(subject, recurring_state);
            let raw = execute_perplexity(config, &prompt).await?;
            let response = parse_recurring_response(&raw)?;
            Ok(ClaudeResponse::Recurring(response))
        }
    }
}
