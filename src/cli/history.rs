use crate::config;
use crate::error::{HeadsupError, Result};
use crate::state::{self, HistoryEntry, SubjectState};
use crate::ui;

/// Run the history command
pub fn run_history(subject_key: Option<String>, limit: usize, json_output: bool) -> Result<()> {
    let config = config::load_config()?;
    let state = state::load_state_readonly()?;

    // Collect history entries
    let mut entries: Vec<(String, &HistoryEntry)> = Vec::new();

    match subject_key {
        Some(key) => {
            // Get history for specific subject
            let subject = config.find_subject(&key)
                .ok_or_else(|| HeadsupError::SubjectNotFound(key.clone()))?;

            if let Some(subject_state) = state.subjects.get(&subject.id) {
                let history = get_history_from_state(subject_state);
                for entry in history.iter().rev().take(limit) {
                    entries.push((subject.name.clone(), entry));
                }
            }
        }
        None => {
            // Get history for all subjects
            for subject in &config.subjects {
                if let Some(subject_state) = state.subjects.get(&subject.id) {
                    let history = get_history_from_state(subject_state);
                    for entry in history {
                        entries.push((subject.name.clone(), entry));
                    }
                }
            }
            // Sort by timestamp descending
            entries.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
            entries.truncate(limit);
        }
    }

    if entries.is_empty() {
        ui::print_info("No history entries found");
        return Ok(());
    }

    if json_output {
        // Output as JSON
        let json_entries: Vec<serde_json::Value> = entries.iter()
            .map(|(name, entry)| {
                serde_json::json!({
                    "subject": name,
                    "timestamp": entry.timestamp,
                    "event": entry.event,
                    "details": entry.details,
                    "source_url": entry.source_url,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_entries).unwrap());
    } else {
        // Output as text
        println!("{:<20} {:<20} {:<15} {}", "TIMESTAMP", "SUBJECT", "EVENT", "DETAILS");
        println!("{}", "-".repeat(80));

        for (name, entry) in entries {
            let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M");
            let details = format_details(&entry.details);
            println!(
                "{:<20} {:<20} {:<15} {}",
                timestamp,
                truncate(&name, 18),
                entry.event,
                truncate(&details, 30)
            );
        }
    }

    Ok(())
}

fn get_history_from_state(state: &SubjectState) -> &[HistoryEntry] {
    match state {
        SubjectState::Release(s) => &s.history,
        SubjectState::Question(s) => &s.history,
        SubjectState::Recurring(s) => &s.history,
    }
}

fn format_details(details: &serde_json::Value) -> String {
    if let Some(obj) = details.as_object() {
        let mut parts: Vec<String> = Vec::new();

        // Extract key information based on content
        if let Some(date) = obj.get("found_release_date").and_then(|v| v.as_str()) {
            parts.push(format!("date: {}", date));
        }
        if let Some(answer) = obj.get("found_answer").and_then(|v| v.as_str()) {
            parts.push(format!("answer: {}", truncate(answer, 20)));
        }
        if let Some(date) = obj.get("next_occurrence_date").and_then(|v| v.as_str()) {
            parts.push(format!("next: {}", date));
        }
        if let Some(notify) = obj.get("should_notify").and_then(|v| v.as_bool()) {
            if notify {
                parts.push("notified".to_string());
            }
        }

        if parts.is_empty() {
            details.to_string()
        } else {
            parts.join(", ")
        }
    } else {
        details.to_string()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
