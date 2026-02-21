use crate::error::{HeadsupError, Result};
use crate::state::{Confidence, DatePrecision, ReleaseStatus};
use serde::{Deserialize, Serialize};

/// Response from Claude for release-type subjects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseResponse {
    pub subject: String,
    pub found_release_date: Option<String>,
    pub release_date_precision: DatePrecision,
    pub confidence: Confidence,
    pub status: ReleaseStatus,
    pub summary: String,
    pub source_url: Option<String>,
    pub source_name: Option<String>,
    pub should_notify: bool,
    pub notify_reason: Option<String>,
}

/// Response from Claude for question-type subjects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResponse {
    pub question: String,
    pub found_answer: Option<String>,
    pub confidence: Confidence,
    pub is_definitive: bool,
    pub summary: String,
    pub source_url: Option<String>,
    pub source_name: Option<String>,
    pub should_notify: bool,
    pub notify_reason: Option<String>,
}

/// Response from Claude for recurring-type subjects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringResponse {
    pub event_name: String,
    pub next_occurrence_date: Option<String>,
    pub next_occurrence_name: Option<String>,
    pub date_precision: DatePrecision,
    pub confidence: Confidence,
    pub summary: String,
    pub source_url: Option<String>,
    pub source_name: Option<String>,
    pub should_notify: bool,
    pub notify_reason: Option<String>,
}

/// Response from Claude for subject identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectIdentificationResponse {
    pub matches: Vec<SubjectMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectMatch {
    pub name: String,
    pub description: String,
    pub suggested_type: Option<String>,
    pub category: Option<String>,
    pub search_terms: Vec<String>,
    pub notes: Option<String>,
    pub question: Option<String>,
    pub event_name: Option<String>,
}

/// Parsed response from Claude (any type)
#[derive(Debug, Clone)]
pub enum ClaudeResponse {
    Release(ReleaseResponse),
    Question(QuestionResponse),
    Recurring(RecurringResponse),
}

/// Parse a release response from Claude's raw output
pub fn parse_release_response(raw: &str) -> Result<ReleaseResponse> {
    let json_str = extract_json(raw)?;
    serde_json::from_str(&json_str)
        .map_err(|e| HeadsupError::ClaudeParseError(format!("Invalid release response: {}", e)))
}

/// Parse a question response from Claude's raw output
pub fn parse_question_response(raw: &str) -> Result<QuestionResponse> {
    let json_str = extract_json(raw)?;
    serde_json::from_str(&json_str)
        .map_err(|e| HeadsupError::ClaudeParseError(format!("Invalid question response: {}", e)))
}

/// Parse a recurring response from Claude's raw output
pub fn parse_recurring_response(raw: &str) -> Result<RecurringResponse> {
    let json_str = extract_json(raw)?;
    serde_json::from_str(&json_str)
        .map_err(|e| HeadsupError::ClaudeParseError(format!("Invalid recurring response: {}", e)))
}

/// Parse a subject identification response from Claude's raw output
pub fn parse_subject_identification_response(raw: &str) -> Result<SubjectIdentificationResponse> {
    let json_str = extract_json(raw)?;
    serde_json::from_str(&json_str)
        .map_err(|e| HeadsupError::ClaudeParseError(format!("Invalid subject identification response: {}", e)))
}

/// Extract JSON from Claude's response, handling potential markdown code blocks
fn extract_json(raw: &str) -> Result<String> {
    let trimmed = raw.trim();

    // Try to extract from markdown code block
    if trimmed.starts_with("```") {
        // Find the end of the opening fence
        if let Some(start) = trimmed.find('\n') {
            let rest = &trimmed[start + 1..];
            // Find the closing fence
            if let Some(end) = rest.rfind("```") {
                return Ok(rest[..end].trim().to_string());
            }
        }
    }

    // Try to find JSON object or array
    if let Some(start) = trimmed.find('{') {
        // Find matching closing brace
        let mut depth = 0;
        let mut end = None;
        for (i, c) in trimmed[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end_pos) = end {
            return Ok(trimmed[start..end_pos].to_string());
        }
    }

    // If all else fails, return the trimmed string and let serde handle it
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    #[test]
    fn test_extract_json_plain() {
        let input = r#"{"subject": "test", "found_release_date": null}"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("subject"));
    }

    #[test]
    fn test_extract_json_markdown() {
        let input = r#"```json
{"subject": "test", "found_release_date": null}
```"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("subject"));
    }

    #[test]
    fn test_extract_json_with_text() {
        let input = r#"Here is the response:
{"subject": "test", "found_release_date": null}
End of response."#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("subject"));
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com"));
        assert!(validate_url("http://example.com"));
        assert!(!validate_url("ftp://example.com"));
        assert!(!validate_url("example.com"));
    }
}
