use crate::error::{HeadsupError, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, Select, Text};
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Global quiet mode flag - when true, suppresses non-error output
static QUIET_MODE: AtomicBool = AtomicBool::new(false);

/// Enable or disable quiet mode globally
pub fn set_quiet_mode(quiet: bool) {
    QUIET_MODE.store(quiet, Ordering::SeqCst);
}

/// Check if quiet mode is enabled
pub fn is_quiet() -> bool {
    QUIET_MODE.load(Ordering::SeqCst)
}

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

/// Print a success message (suppressed in quiet mode)
pub fn print_success(message: &str) {
    if !is_quiet() {
        println!("{} {}", style("✓").green(), message);
    }
}

/// Print an error message (always shown, even in quiet mode)
pub fn print_error(message: &str) {
    eprintln!("{} {}", style("✗").red(), message);
}

/// Print a warning message (suppressed in quiet mode)
pub fn print_warning(message: &str) {
    if !is_quiet() {
        eprintln!("{} {}", style("!").yellow(), message);
    }
}

/// Print an info message (suppressed in quiet mode)
pub fn print_info(message: &str) {
    if !is_quiet() {
        println!("{} {}", style("→").blue(), message);
    }
}

/// Print a blank line (suppressed in quiet mode)
pub fn print_blank() {
    if !is_quiet() {
        println!();
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
