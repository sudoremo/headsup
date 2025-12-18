use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub email: EmailConfig,
    pub claude: ClaudeConfig,
    pub settings: Settings,
    #[serde(default)]
    pub subjects: Vec<Subject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub to: String,
    pub from: String,
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    pub smtp_username: String,
    /// Command to execute to retrieve the SMTP password
    pub smtp_password_command: String,
    #[serde(default = "default_smtp_timeout")]
    pub smtp_timeout_seconds: u64,
    #[serde(default)]
    pub digest_mode: bool,
}

fn default_smtp_port() -> u16 {
    587
}

fn default_smtp_timeout() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    #[serde(default = "default_claude_command")]
    pub command: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_searches")]
    pub max_searches_per_run: u32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_failures")]
    pub max_consecutive_failures: u32,
    #[serde(default)]
    pub total_run_timeout_seconds: u64,
}

fn default_claude_command() -> String {
    "claude".to_string()
}

fn default_model() -> String {
    "sonnet".to_string()
}

fn default_max_searches() -> u32 {
    20
}

fn default_timeout() -> u64 {
    60
}

fn default_max_failures() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    #[serde(default = "default_log_format")]
    pub log_format: LogFormat,
    #[serde(default = "default_imminent_days")]
    pub imminent_threshold_days: u32,
    #[serde(default = "default_max_history")]
    pub max_history_entries: u32,
}

fn default_log_level() -> LogLevel {
    LogLevel::Quiet
}

fn default_log_format() -> LogFormat {
    LogFormat::Text
}

fn default_imminent_days() -> u32 {
    7
}

fn default_max_history() -> u32 {
    50
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Quiet,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub key: String,
    pub name: String,
    #[serde(default, rename = "type")]
    pub subject_type: SubjectType,
    #[serde(default)]
    pub category: Option<Category>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub event_name: Option<String>,
    pub search_terms: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl Subject {
    /// Generate a key from the subject name
    pub fn generate_key(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
            .chars()
            .take(32)
            .collect()
    }

    /// Validate subject configuration based on type
    pub fn validate(&self) -> Result<(), String> {
        // Key validation
        if !self.key.is_empty() {
            if self.key.len() > 32 {
                return Err("Key must be 32 characters or less".to_string());
            }
            if self.key.starts_with('-') || self.key.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                return Err("Key cannot start with a number or hyphen".to_string());
            }
            if !self.key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err("Key must contain only lowercase letters, numbers, and hyphens".to_string());
            }
        }

        // Type-specific validation
        match self.subject_type {
            SubjectType::Release => {
                if self.category.is_none() {
                    return Err("Release type subjects require a category".to_string());
                }
            }
            SubjectType::Question => {
                if self.question.is_none() || self.question.as_ref().map_or(true, |q| q.is_empty()) {
                    return Err("Question type subjects require a question field".to_string());
                }
            }
            SubjectType::Recurring => {
                if self.event_name.is_none() || self.event_name.as_ref().map_or(true, |e| e.is_empty()) {
                    return Err("Recurring type subjects require an event_name field".to_string());
                }
            }
        }

        if self.search_terms.is_empty() {
            return Err("At least one search term is required".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubjectType {
    #[default]
    Release,
    Question,
    Recurring,
}

impl std::fmt::Display for SubjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubjectType::Release => write!(f, "release"),
            SubjectType::Question => write!(f, "question"),
            SubjectType::Recurring => write!(f, "recurring"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Game,
    TvShow,
    TvSeason,
    Movie,
    Software,
    Other,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Game => write!(f, "Game"),
            Category::TvShow => write!(f, "TV Show"),
            Category::TvSeason => write!(f, "TV Season"),
            Category::Movie => write!(f, "Movie"),
            Category::Software => write!(f, "Software"),
            Category::Other => write!(f, "Other"),
        }
    }
}

impl Config {
    /// Create a default config with placeholder values
    pub fn default_with_email(email: &str) -> Self {
        Config {
            email: EmailConfig {
                to: email.to_string(),
                from: format!("radar@{}", email.split('@').nth(1).unwrap_or("example.com")),
                smtp_host: "smtp.example.com".to_string(),
                smtp_port: 587,
                smtp_username: "user".to_string(),
                smtp_password_command: "echo 'your-password-here'".to_string(),
                smtp_timeout_seconds: 30,
                digest_mode: false,
            },
            claude: ClaudeConfig {
                command: "claude".to_string(),
                model: "sonnet".to_string(),
                max_searches_per_run: 20,
                timeout_seconds: 60,
                max_consecutive_failures: 3,
                total_run_timeout_seconds: 600,
            },
            settings: Settings {
                log_level: LogLevel::Quiet,
                log_format: LogFormat::Text,
                imminent_threshold_days: 7,
                max_history_entries: 50,
            },
            subjects: vec![],
        }
    }

    /// Find a subject by key or UUID
    pub fn find_subject(&self, key_or_id: &str) -> Option<&Subject> {
        // Try UUID first
        if let Ok(uuid) = Uuid::parse_str(key_or_id) {
            return self.subjects.iter().find(|s| s.id == uuid);
        }
        // Then try key (case-insensitive)
        let lower = key_or_id.to_lowercase();
        self.subjects.iter().find(|s| s.key.to_lowercase() == lower)
    }

    /// Find a subject mutably by key or UUID
    pub fn find_subject_mut(&mut self, key_or_id: &str) -> Option<&mut Subject> {
        // Try UUID first
        if let Ok(uuid) = Uuid::parse_str(key_or_id) {
            return self.subjects.iter_mut().find(|s| s.id == uuid);
        }
        // Then try key (case-insensitive)
        let lower = key_or_id.to_lowercase();
        self.subjects.iter_mut().find(|s| s.key.to_lowercase() == lower)
    }

    /// Check if a key is already in use
    pub fn key_exists(&self, key: &str) -> bool {
        let lower = key.to_lowercase();
        self.subjects.iter().any(|s| s.key.to_lowercase() == lower)
    }

    /// Generate a unique key for a subject name
    pub fn generate_unique_key(&self, name: &str) -> String {
        let base_key = Subject::generate_key(name);
        if !self.key_exists(&base_key) {
            return base_key;
        }

        for i in 2..=100 {
            let candidate = format!("{}-{}", base_key.chars().take(28).collect::<String>(), i);
            if !self.key_exists(&candidate) {
                return candidate;
            }
        }

        // Fallback to UUID-based key
        format!("{}-{}", base_key.chars().take(24).collect::<String>(), &Uuid::new_v4().to_string()[..7])
    }

    /// Validate the entire configuration
    pub fn validate(&self) -> Result<Vec<String>, Vec<String>> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Validate email config
        if self.email.to.is_empty() {
            errors.push("Email 'to' address is required".to_string());
        }
        if self.email.smtp_host.is_empty() {
            errors.push("SMTP host is required".to_string());
        }

        // Validate subjects
        let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (i, subject) in self.subjects.iter().enumerate() {
            // Check for duplicate keys
            let key_lower = subject.key.to_lowercase();
            if !key_lower.is_empty() && !seen_keys.insert(key_lower.clone()) {
                errors.push(format!("Duplicate subject key: {}", subject.key));
            }

            // Validate subject
            if let Err(e) = subject.validate() {
                errors.push(format!("Subject '{}' (index {}): {}", subject.name, i, e));
            }
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }
}
