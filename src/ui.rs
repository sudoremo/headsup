use crate::error::{HeadsupError, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, Select, Text};
use std::fmt::Display;
use std::time::Duration;

/// Prompt for text input
pub fn prompt_text(message: &str) -> Result<String> {
    Text::new(message)
        .prompt()
        .map_err(|_| HeadsupError::UserCancelled)
}

/// Prompt for text input with default value
pub fn prompt_text_with_default(message: &str, default: &str) -> Result<String> {
    Text::new(message)
        .with_default(default)
        .prompt()
        .map_err(|_| HeadsupError::UserCancelled)
}

/// Prompt for confirmation
pub fn prompt_confirm(message: &str, default: bool) -> Result<bool> {
    Confirm::new(message)
        .with_default(default)
        .prompt()
        .map_err(|_| HeadsupError::UserCancelled)
}

/// Prompt for selection from a list
pub fn prompt_select<T: Display>(message: &str, options: Vec<T>) -> Result<T> {
    Select::new(message, options)
        .prompt()
        .map_err(|_| HeadsupError::UserCancelled)
}

/// Prompt for selection from a list, returning the index
pub fn prompt_select_index<T: Display>(message: &str, options: &[T]) -> Result<usize> {
    let options_vec: Vec<String> = options.iter().map(|o| o.to_string()).collect();
    let selected = Select::new(message, options_vec)
        .prompt()
        .map_err(|_| HeadsupError::UserCancelled)?;

    // Find the index of the selected option
    options.iter()
        .position(|o| o.to_string() == selected)
        .ok_or_else(|| HeadsupError::UserCancelled)
}

/// Create a spinner with a message
pub struct Spinner {
    progress: ProgressBar,
}

impl Spinner {
    /// Create and start a new spinner
    pub fn new(message: &str) -> Self {
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        progress.set_message(message.to_string());
        progress.enable_steady_tick(Duration::from_millis(100));
        Spinner { progress }
    }

    /// Update the spinner message
    pub fn set_message(&self, message: &str) {
        self.progress.set_message(message.to_string());
    }

    /// Stop the spinner with a success message
    pub fn finish_with_message(&self, message: &str) {
        self.progress.finish_with_message(format!("{} {}", style("✓").green(), message));
    }

    /// Stop the spinner with an error message
    pub fn finish_with_error(&self, message: &str) {
        self.progress.finish_with_message(format!("{} {}", style("✗").red(), message));
    }

    /// Stop the spinner and clear it
    pub fn finish_and_clear(&self) {
        self.progress.finish_and_clear();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if !self.progress.is_finished() {
            self.progress.finish_and_clear();
        }
    }
}

/// Execute a function while showing a spinner
pub fn with_spinner<F, T>(message: &str, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let spinner = Spinner::new(message);
    let result = f();
    match &result {
        Ok(_) => spinner.finish_and_clear(),
        Err(e) => spinner.finish_with_error(&e.to_string()),
    }
    result
}

/// Execute an async function while showing a spinner
pub async fn with_spinner_async<F, T, Fut>(message: &str, f: F) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let spinner = Spinner::new(message);
    let result = f().await;
    match &result {
        Ok(_) => spinner.finish_and_clear(),
        Err(e) => spinner.finish_with_error(&e.to_string()),
    }
    result
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("{} {}", style("✓").green(), message);
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", style("✗").red(), message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    eprintln!("{} {}", style("!").yellow(), message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{} {}", style("→").blue(), message);
}

/// Format a subject type for display
pub fn format_subject_type(subject_type: &crate::config::SubjectType) -> String {
    match subject_type {
        crate::config::SubjectType::Release => "Release date (one-time)".to_string(),
        crate::config::SubjectType::Question => "General question".to_string(),
        crate::config::SubjectType::Recurring => "Recurring event".to_string(),
    }
}

/// Format a category for display
pub fn format_category(category: &crate::config::Category) -> String {
    match category {
        crate::config::Category::Game => "Game".to_string(),
        crate::config::Category::TvShow => "TV Show".to_string(),
        crate::config::Category::TvSeason => "TV Season".to_string(),
        crate::config::Category::Movie => "Movie".to_string(),
        crate::config::Category::Music => "Music".to_string(),
        crate::config::Category::Software => "Software".to_string(),
        crate::config::Category::Other => "Other".to_string(),
    }
}

/// Selection options for subject type
pub fn subject_type_options() -> Vec<&'static str> {
    vec![
        "Release date (one-time)",
        "General question",
        "Recurring event",
    ]
}

/// Parse selected subject type option to SubjectType
pub fn parse_subject_type_option(option: &str) -> crate::config::SubjectType {
    match option {
        "Release date (one-time)" => crate::config::SubjectType::Release,
        "General question" => crate::config::SubjectType::Question,
        "Recurring event" => crate::config::SubjectType::Recurring,
        _ => crate::config::SubjectType::Release,
    }
}

/// Selection options for category
pub fn category_options() -> Vec<&'static str> {
    vec!["Game", "TV Show", "TV Season", "Movie", "Music", "Software", "Other"]
}

/// Parse selected category option to Category
pub fn parse_category_option(option: &str) -> crate::config::Category {
    match option {
        "Game" => crate::config::Category::Game,
        "TV Show" => crate::config::Category::TvShow,
        "TV Season" => crate::config::Category::TvSeason,
        "Movie" => crate::config::Category::Movie,
        "Music" => crate::config::Category::Music,
        "Software" => crate::config::Category::Software,
        "Other" => crate::config::Category::Other,
        _ => crate::config::Category::Other,
    }
}

/// Check if running in a TTY
pub fn is_interactive() -> bool {
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}
