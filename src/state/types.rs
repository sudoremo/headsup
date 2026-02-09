use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The current state file version
pub const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub version: u32,
    pub last_run: Option<DateTime<Utc>>,
    #[serde(default)]
    pub subjects: HashMap<Uuid, SubjectState>,
    #[serde(default)]
    pub pending_notifications: Vec<PendingNotification>,
}

impl Default for State {
    fn default() -> Self {
        State {
            version: STATE_VERSION,
            last_run: None,
            subjects: HashMap::new(),
            pending_notifications: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SubjectState {
    Release(ReleaseState),
    Question(QuestionState),
    Recurring(RecurringState),
}

impl SubjectState {
    pub fn last_checked(&self) -> Option<DateTime<Utc>> {
        match self {
            SubjectState::Release(s) => s.last_checked,
            SubjectState::Question(s) => s.last_checked,
            SubjectState::Recurring(s) => s.last_checked,
        }
    }

    pub fn consecutive_failures(&self) -> u32 {
        match self {
            SubjectState::Release(s) => s.consecutive_failures,
            SubjectState::Question(s) => s.consecutive_failures,
            SubjectState::Recurring(s) => s.consecutive_failures,
        }
    }

    pub fn increment_failure(&mut self, reason: &str) {
        match self {
            SubjectState::Release(s) => {
                s.consecutive_failures += 1;
                s.last_failure_reason = Some(reason.to_string());
                s.last_failure_time = Some(Utc::now());
            }
            SubjectState::Question(s) => {
                s.consecutive_failures += 1;
                s.last_failure_reason = Some(reason.to_string());
                s.last_failure_time = Some(Utc::now());
            }
            SubjectState::Recurring(s) => {
                s.consecutive_failures += 1;
                s.last_failure_reason = Some(reason.to_string());
                s.last_failure_time = Some(Utc::now());
            }
        }
    }

    pub fn last_failure_time(&self) -> Option<DateTime<Utc>> {
        match self {
            SubjectState::Release(s) => s.last_failure_time,
            SubjectState::Question(s) => s.last_failure_time,
            SubjectState::Recurring(s) => s.last_failure_time,
        }
    }

    pub fn reset_failures(&mut self) {
        match self {
            SubjectState::Release(s) => {
                s.consecutive_failures = 0;
                s.last_failure_reason = None;
                s.last_failure_time = None;
            }
            SubjectState::Question(s) => {
                s.consecutive_failures = 0;
                s.last_failure_reason = None;
                s.last_failure_time = None;
            }
            SubjectState::Recurring(s) => {
                s.consecutive_failures = 0;
                s.last_failure_reason = None;
                s.last_failure_time = None;
            }
        }
    }

    pub fn set_last_checked(&mut self, time: DateTime<Utc>) {
        match self {
            SubjectState::Release(s) => s.last_checked = Some(time),
            SubjectState::Question(s) => s.last_checked = Some(time),
            SubjectState::Recurring(s) => s.last_checked = Some(time),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseState {
    pub last_checked: Option<DateTime<Utc>>,
    pub known_release_date: Option<String>,
    pub release_date_precision: DatePrecision,
    pub confidence: Confidence,
    pub status: ReleaseStatus,
    pub last_notified: Option<DateTime<Utc>>,
    pub imminent_notified: bool,
    pub consecutive_failures: u32,
    pub last_failure_reason: Option<String>,
    #[serde(default)]
    pub last_failure_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

impl Default for ReleaseState {
    fn default() -> Self {
        ReleaseState {
            last_checked: None,
            known_release_date: None,
            release_date_precision: DatePrecision::Unknown,
            confidence: Confidence::Unknown,
            status: ReleaseStatus::Unknown,
            last_notified: None,
            imminent_notified: false,
            consecutive_failures: 0,
            last_failure_reason: None,
            last_failure_time: None,
            history: Vec::new(),
        }
    }
}

impl ReleaseState {
    pub fn reset_failures(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure_reason = None;
        self.last_failure_time = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionState {
    pub last_checked: Option<DateTime<Utc>>,
    pub current_answer: Option<String>,
    pub confidence: Confidence,
    pub is_definitive: bool,
    pub last_notified: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub last_failure_reason: Option<String>,
    #[serde(default)]
    pub last_failure_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

impl Default for QuestionState {
    fn default() -> Self {
        QuestionState {
            last_checked: None,
            current_answer: None,
            confidence: Confidence::Unknown,
            is_definitive: false,
            last_notified: None,
            consecutive_failures: 0,
            last_failure_reason: None,
            last_failure_time: None,
            history: Vec::new(),
        }
    }
}

impl QuestionState {
    pub fn reset_failures(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure_reason = None;
        self.last_failure_time = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringState {
    pub last_checked: Option<DateTime<Utc>>,
    pub next_occurrence_date: Option<String>,
    pub next_occurrence_name: Option<String>,
    pub date_precision: DatePrecision,
    pub confidence: Confidence,
    pub last_occurrence_date: Option<String>,
    pub occurrence_count: u32,
    pub last_notified: Option<DateTime<Utc>>,
    pub imminent_notified: bool,
    pub consecutive_failures: u32,
    pub last_failure_reason: Option<String>,
    #[serde(default)]
    pub last_failure_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

impl Default for RecurringState {
    fn default() -> Self {
        RecurringState {
            last_checked: None,
            next_occurrence_date: None,
            next_occurrence_name: None,
            date_precision: DatePrecision::Unknown,
            confidence: Confidence::Unknown,
            last_occurrence_date: None,
            occurrence_count: 0,
            last_notified: None,
            imminent_notified: false,
            consecutive_failures: 0,
            last_failure_reason: None,
            last_failure_time: None,
            history: Vec::new(),
        }
    }
}

impl RecurringState {
    pub fn reset_failures(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure_reason = None;
        self.last_failure_time = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    #[serde(flatten)]
    pub details: serde_json::Value,
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DatePrecision {
    Exact,
    Month,
    Season,
    Year,
    #[default]
    Unknown,
}

impl DatePrecision {
    /// Check if this precision is more precise than another
    pub fn is_more_precise_than(&self, other: &DatePrecision) -> bool {
        let self_rank = self.rank();
        let other_rank = other.rank();
        self_rank < other_rank
    }

    fn rank(&self) -> u8 {
        match self {
            DatePrecision::Exact => 1,
            DatePrecision::Month => 2,
            DatePrecision::Season => 3,
            DatePrecision::Year => 4,
            DatePrecision::Unknown => 5,
        }
    }
}

impl std::fmt::Display for DatePrecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatePrecision::Exact => write!(f, "exact"),
            DatePrecision::Month => write!(f, "month"),
            DatePrecision::Season => write!(f, "season"),
            DatePrecision::Year => write!(f, "year"),
            DatePrecision::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Official,
    Reliable,
    Rumor,
    Speculation,
    #[default]
    Unknown,
}

impl Confidence {
    /// Check if this confidence is higher than another
    pub fn is_higher_than(&self, other: &Confidence) -> bool {
        let self_rank = self.rank();
        let other_rank = other.rank();
        self_rank < other_rank
    }

    fn rank(&self) -> u8 {
        match self {
            Confidence::Official => 1,
            Confidence::Reliable => 2,
            Confidence::Rumor => 3,
            Confidence::Speculation => 4,
            Confidence::Unknown => 5,
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::Official => write!(f, "Official announcement"),
            Confidence::Reliable => write!(f, "Reliable sources"),
            Confidence::Rumor => write!(f, "Rumor"),
            Confidence::Speculation => write!(f, "Speculation"),
            Confidence::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseStatus {
    Announced,
    Delayed,
    Released,
    Cancelled,
    #[default]
    Unknown,
}

impl std::fmt::Display for ReleaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseStatus::Announced => write!(f, "Announced"),
            ReleaseStatus::Delayed => write!(f, "Delayed"),
            ReleaseStatus::Released => write!(f, "Released"),
            ReleaseStatus::Cancelled => write!(f, "Cancelled"),
            ReleaseStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingNotification {
    pub subject_id: Uuid,
    pub event_type: String,
    pub created_at: DateTime<Utc>,
    pub summary: String,
    pub source_url: Option<String>,
    pub payload: serde_json::Value,
}

impl State {
    /// Get or create state for a subject
    pub fn get_or_create_release(&mut self, id: Uuid) -> &mut ReleaseState {
        self.subjects.entry(id).or_insert_with(|| SubjectState::Release(ReleaseState::default()));
        match self.subjects.get_mut(&id).unwrap() {
            SubjectState::Release(state) => state,
            _ => panic!("Subject type mismatch"),
        }
    }

    /// Get or create state for a question subject
    pub fn get_or_create_question(&mut self, id: Uuid) -> &mut QuestionState {
        self.subjects.entry(id).or_insert_with(|| SubjectState::Question(QuestionState::default()));
        match self.subjects.get_mut(&id).unwrap() {
            SubjectState::Question(state) => state,
            _ => panic!("Subject type mismatch"),
        }
    }

    /// Get or create state for a recurring subject
    pub fn get_or_create_recurring(&mut self, id: Uuid) -> &mut RecurringState {
        self.subjects.entry(id).or_insert_with(|| SubjectState::Recurring(RecurringState::default()));
        match self.subjects.get_mut(&id).unwrap() {
            SubjectState::Recurring(state) => state,
            _ => panic!("Subject type mismatch"),
        }
    }

    /// Prune orphaned subjects (not in config)
    pub fn prune_orphans(&mut self, valid_ids: &[Uuid]) -> Vec<Uuid> {
        let orphans: Vec<Uuid> = self.subjects
            .keys()
            .filter(|id| !valid_ids.contains(id))
            .copied()
            .collect();

        for id in &orphans {
            self.subjects.remove(id);
        }

        orphans
    }

    /// Add a history entry for a subject
    pub fn add_history(&mut self, id: Uuid, entry: HistoryEntry, max_entries: u32) {
        if let Some(state) = self.subjects.get_mut(&id) {
            let history = match state {
                SubjectState::Release(s) => &mut s.history,
                SubjectState::Question(s) => &mut s.history,
                SubjectState::Recurring(s) => &mut s.history,
            };

            history.push(entry);

            // Prune old entries
            while history.len() > max_entries as usize {
                history.remove(0);
            }
        }
    }

    /// Clear pending notifications
    pub fn clear_pending_notifications(&mut self) -> Vec<PendingNotification> {
        std::mem::take(&mut self.pending_notifications)
    }

    /// Add a pending notification
    pub fn add_pending_notification(&mut self, notification: PendingNotification) {
        self.pending_notifications.push(notification);
    }
}
