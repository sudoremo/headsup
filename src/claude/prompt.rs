use crate::config::{Category, Subject, SubjectType};
use crate::state::{Confidence, DatePrecision, QuestionState, RecurringState, ReleaseState};

/// Build the prompt for a release-type subject
pub fn build_release_prompt(subject: &Subject, state: Option<&ReleaseState>) -> String {
    let category = subject.category.as_ref().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
    let search_terms_section = if subject.search_terms.is_empty() {
        String::new()
    } else {
        format!("SEARCH TERMS: {}\n", subject.search_terms.join(", "))
    };

    let state_info = if let Some(s) = state {
        if let Some(ref date) = s.known_release_date {
            format!(
                "CURRENT KNOWN STATE:\n- Release Date: {} ({}, {})\n- Status: {}",
                date, s.release_date_precision, s.confidence, s.status
            )
        } else {
            "CURRENT KNOWN STATE:\n- No release date currently known".to_string()
        }
    } else {
        "CURRENT KNOWN STATE:\n- No release date currently known".to_string()
    };

    let notes_section = subject.notes.as_ref()
        .map(|n| format!("CONTEXT: {}\n", n))
        .unwrap_or_default();

    format!(r#"You are analyzing release date information for a tracked subject.

SUBJECT: {name}
CATEGORY: {category}
{search_terms_section}{notes_section}
{state_info}

TASK:
1. Search for recent news about this subject's release date
2. Evaluate the credibility of sources (official > major outlets > rumors)
3. Compare findings to the current known state
4. Determine if the user should be notified

Return a JSON response with this exact structure:
{{
  "subject": "{name}",
  "found_release_date": "string or null",
  "release_date_precision": "exact|month|season|year|unknown",
  "confidence": "official|reliable|rumor|speculation|unknown",
  "status": "announced|delayed|released|cancelled|unknown",
  "summary": "Brief description of findings",
  "source_url": "URL of most credible source or null",
  "source_name": "Name of source (e.g., 'Official PlayStation Blog')",
  "should_notify": true/false,
  "notify_reason": "Why user should be notified (or null)"
}}

NOTIFICATION CRITERIA:
- Notify if: new release date announced, date changed, release imminent (within 7 days), or release happened
- Do NOT notify if: only rumors with no credible source, information unchanged, already notified for this event

Respond with ONLY the JSON object, no other text."#,
        name = subject.name,
        category = category,
        search_terms_section = search_terms_section,
        notes_section = notes_section,
        state_info = state_info
    )
}

/// Build the prompt for a question-type subject
pub fn build_question_prompt(subject: &Subject, state: Option<&QuestionState>) -> String {
    let question = subject.question.as_ref().map(|q| q.as_str()).unwrap_or("Unknown question");
    let search_terms_section = if subject.search_terms.is_empty() {
        String::new()
    } else {
        format!("SEARCH TERMS: {}\n", subject.search_terms.join(", "))
    };

    let state_info = if let Some(s) = state {
        if let Some(ref answer) = s.current_answer {
            format!(
                "CURRENT KNOWN STATE:\n- Current Answer: {} ({})\n- Is Definitive: {}",
                answer, s.confidence, s.is_definitive
            )
        } else {
            "CURRENT KNOWN STATE:\n- No answer currently known".to_string()
        }
    } else {
        "CURRENT KNOWN STATE:\n- No answer currently known".to_string()
    };

    let notes_section = subject.notes.as_ref()
        .map(|n| format!("CONTEXT: {}\n", n))
        .unwrap_or_default();

    format!(r#"You are researching an answer to a tracked question.

QUESTION: {question}
{search_terms_section}{notes_section}
{state_info}

TASK:
1. Search for recent information about this question
2. Evaluate the credibility of sources
3. Determine if there's a definitive/official answer
4. Compare findings to the current known state
5. Determine if the user should be notified

Return a JSON response with this exact structure:
{{
  "question": "{question}",
  "found_answer": "string or null",
  "confidence": "official|reliable|rumor|speculation|unknown",
  "is_definitive": true/false,
  "summary": "Brief description of findings",
  "source_url": "URL of most credible source or null",
  "source_name": "Name of source",
  "should_notify": true/false,
  "notify_reason": "Why user should be notified (or null)"
}}

NOTIFICATION CRITERIA:
- Notify if: new answer found, answer changed, confidence upgraded to official/reliable, answer confirmed as definitive
- Do NOT notify if: only speculation, same answer with same confidence, already notified for this

Respond with ONLY the JSON object, no other text."#,
        question = question,
        search_terms_section = search_terms_section,
        notes_section = notes_section,
        state_info = state_info
    )
}

/// Build the prompt for a recurring-type subject
pub fn build_recurring_prompt(subject: &Subject, state: Option<&RecurringState>) -> String {
    let event_name = subject.event_name.as_ref().map(|e| e.as_str()).unwrap_or("Unknown event");
    let search_terms_section = if subject.search_terms.is_empty() {
        String::new()
    } else {
        format!("SEARCH TERMS: {}\n", subject.search_terms.join(", "))
    };

    let state_info = if let Some(s) = state {
        let mut info = String::from("CURRENT KNOWN STATE:\n");
        if let Some(ref date) = s.next_occurrence_date {
            info.push_str(&format!("- Next Event: {} ({})\n", date, s.date_precision));
            if let Some(ref name) = s.next_occurrence_name {
                info.push_str(&format!("- Event Name: {}\n", name));
            }
        } else {
            info.push_str("- No next event date currently known\n");
        }
        if let Some(ref last) = s.last_occurrence_date {
            info.push_str(&format!("- Last Event: {}\n", last));
        }
        info.push_str(&format!("- Total Events Tracked: {}", s.occurrence_count));
        info
    } else {
        "CURRENT KNOWN STATE:\n- No event information currently known".to_string()
    };

    let notes_section = subject.notes.as_ref()
        .map(|n| format!("CONTEXT: {}\n", n))
        .unwrap_or_default();

    format!(r#"You are researching the next occurrence of a recurring event.

EVENT: {event_name}
{search_terms_section}{notes_section}
{state_info}

TASK:
1. Search for information about the next upcoming occurrence of this event
2. Evaluate the credibility of sources
3. Compare findings to the current known state
4. Determine if the user should be notified

Return a JSON response with this exact structure:
{{
  "event_name": "{event_name}",
  "next_occurrence_date": "string (YYYY-MM-DD format if known) or null",
  "next_occurrence_name": "Specific name of event (e.g., 'WWDC 2025') or null",
  "date_precision": "exact|month|season|year|unknown",
  "confidence": "official|reliable|rumor|speculation|unknown",
  "summary": "Brief description of findings",
  "source_url": "URL of most credible source or null",
  "source_name": "Name of source",
  "should_notify": true/false,
  "notify_reason": "Why user should be notified (or null)"
}}

NOTIFICATION CRITERIA:
- Notify if: next event date found, date changed, event imminent (within 7 days)
- Do NOT notify if: same date and event name, only speculation, already notified for this

Respond with ONLY the JSON object, no other text."#,
        event_name = event_name,
        search_terms_section = search_terms_section,
        notes_section = notes_section,
        state_info = state_info
    )
}

/// Build the prompt for AI-assisted subject addition (does NOT reveal current state)
pub fn build_subject_identification_prompt(user_input: &str) -> String {
    format!(r#"The user wants to add a subject to track for release date monitoring or question answering.

USER INPUT: "{user_input}"

Search for what the user might be referring to. Consider:
- Upcoming games, TV shows, movies, or software
- Announced but unreleased items
- Recurring events (like conferences, keynotes, annual releases)
- Questions about future events or decisions

Return a JSON array of up to 4 possible matches:
{{
  "matches": [
    {{
      "name": "Official title",
      "description": "Brief description (studio, platform, context, etc.)",
      "suggested_type": "release|question|recurring",
      "category": "game|tv_show|tv_season|movie|software|other",
      "search_terms": ["suggested search term 1", "suggested search term 2"],
      "notes": "Any relevant context for tracking",
      "question": "If type is question, the question to track",
      "event_name": "If type is recurring, the event name"
    }}
  ]
}}

If no matches found, return: {{"matches": []}}
Respond with ONLY the JSON object, no other text."#,
        user_input = user_input
    )
}
