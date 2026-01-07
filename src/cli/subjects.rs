use crate::cli::SubjectsCommands;
use crate::claude;
use crate::config::{self, Category, Config, Subject, SubjectType};
use crate::error::{HeadsupError, Result};
use crate::state;
use crate::ui;
use uuid::Uuid;

/// Run subjects subcommands
pub async fn run_subjects(command: SubjectsCommands) -> Result<()> {
    match command {
        SubjectsCommands::List => list_subjects(),
        SubjectsCommands::Add => add_subject().await,
        SubjectsCommands::Remove { key } => remove_subject(&key),
        SubjectsCommands::Edit { key } => edit_subject(&key),
        SubjectsCommands::Enable { key } => enable_subject(&key),
        SubjectsCommands::Disable { key } => disable_subject(&key),
    }
}

fn list_subjects() -> Result<()> {
    let config = config::load_config()?;
    let state = state::load_state_readonly().unwrap_or_default();

    if config.subjects.is_empty() {
        ui::print_info("No subjects configured");
        ui::print_info("Use 'headsup subjects add' to add a subject");
        return Ok(());
    }

    println!("{:<12} {:<30} {:<10} {:<10} {}", "KEY", "NAME", "TYPE", "STATUS", "LAST CHECKED");
    println!("{}", "-".repeat(80));

    for subject in &config.subjects {
        let status = if subject.enabled { "enabled" } else { "disabled" };
        let last_checked = state.subjects.get(&subject.id)
            .and_then(|s| s.last_checked())
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "never".to_string());

        println!(
            "{:<12} {:<30} {:<10} {:<10} {}",
            subject.key,
            truncate(&subject.name, 28),
            subject.subject_type,
            status,
            last_checked
        );
    }

    Ok(())
}

async fn add_subject() -> Result<()> {
    if !ui::is_interactive() {
        return Err(HeadsupError::Config(
            "Interactive mode required for adding subjects. Edit config file directly.".to_string(),
        ));
    }

    let mut config = config::load_config()?;

    // Get user input
    let query = ui::prompt_text("What would you like to track?")?;

    // Use Claude to identify the subject (without revealing current state)
    let spinner = ui::Spinner::new("Searching...");
    let identification = match claude::identify_subjects(&config.claude, &query).await {
        Ok(result) => {
            spinner.finish_and_clear();
            result
        }
        Err(e) => {
            spinner.finish_with_error(&e.to_string());
            // Fall back to manual entry
            ui::print_warning("Could not identify subject automatically. Please enter details manually.");
            return add_subject_manual(&mut config).await;
        }
    };

    if identification.matches.is_empty() {
        ui::print_info("No matches found. Please enter details manually.");
        return add_subject_manual(&mut config).await;
    }

    // Build options for selection
    let mut options: Vec<String> = identification.matches.iter()
        .map(|m| format!("{}\n  {}", m.name, m.description))
        .collect();
    options.push("Something else...".to_string());

    let selected = ui::prompt_select("Did you mean:", options.clone())?;

    if selected == "Something else..." {
        return add_subject_manual(&mut config).await;
    }

    // Find the selected match
    let selected_idx = options.iter().position(|o| o == &selected).unwrap();
    let matched = &identification.matches[selected_idx];

    // Confirm subject type
    let type_options = ui::subject_type_options();
    let suggested_idx = match matched.suggested_type.as_deref() {
        Some("question") => 1,
        Some("recurring") => 2,
        _ => 0,
    };
    let type_selection = ui::prompt_select(
        "What type of tracking?",
        if suggested_idx == 0 {
            type_options.clone()
        } else {
            // Reorder to put suggested first
            let mut reordered = vec![type_options[suggested_idx]];
            for (i, opt) in type_options.iter().enumerate() {
                if i != suggested_idx {
                    reordered.push(opt);
                }
            }
            reordered
        },
    )?;
    let subject_type = ui::parse_subject_type_option(&type_selection);

    // For release type, confirm category
    let category = if subject_type == SubjectType::Release {
        let cat_options = ui::category_options();
        let cat_selection = ui::prompt_select("What category is this?", cat_options)?;
        Some(ui::parse_category_option(&cat_selection))
    } else {
        None
    };

    // For question type, get the question
    let question = if subject_type == SubjectType::Question {
        let default_question = matched.question.clone().unwrap_or_default();
        if default_question.is_empty() {
            Some(ui::prompt_text("What question should be tracked?")?)
        } else {
            Some(ui::prompt_text_with_default("Question to track:", &default_question)?)
        }
    } else {
        None
    };

    // For recurring type, get the event name
    let event_name = if subject_type == SubjectType::Recurring {
        let default_event = matched.event_name.clone().unwrap_or_default();
        if default_event.is_empty() {
            Some(ui::prompt_text("Event name:")?)
        } else {
            Some(ui::prompt_text_with_default("Event name:", &default_event)?)
        }
    } else {
        None
    };

    // Generate key
    let key = config.generate_unique_key(&matched.name);

    // Create subject
    let subject = Subject {
        id: Uuid::new_v4(),
        key,
        name: matched.name.clone(),
        subject_type,
        category,
        question,
        event_name,
        search_terms: matched.search_terms.clone(),
        notes: matched.notes.clone(),
        enabled: true,
    };

    // Validate
    subject.validate().map_err(|e| HeadsupError::Config(e))?;

    // Add to config
    config.subjects.push(subject.clone());
    config::save_config(&config)?;

    ui::print_success(&format!("Added '{}' to your headsup", subject.name));

    Ok(())
}

async fn add_subject_manual(config: &mut Config) -> Result<()> {
    // Get name
    let name = ui::prompt_text("Subject name:")?;

    // Get type
    let type_options = ui::subject_type_options();
    let type_selection = ui::prompt_select("What type of tracking?", type_options)?;
    let subject_type = ui::parse_subject_type_option(&type_selection);

    // Type-specific fields
    let category = if subject_type == SubjectType::Release {
        let cat_options = ui::category_options();
        let cat_selection = ui::prompt_select("Category:", cat_options)?;
        Some(ui::parse_category_option(&cat_selection))
    } else {
        None
    };

    let question = if subject_type == SubjectType::Question {
        Some(ui::prompt_text("Question to track:")?)
    } else {
        None
    };

    let event_name = if subject_type == SubjectType::Recurring {
        Some(ui::prompt_text("Event name:")?)
    } else {
        None
    };

    // Get search terms (optional - AI can determine queries from context)
    let search_terms_input = ui::prompt_text("Search terms (comma-separated, or press Enter to let AI decide):")?;
    let search_terms: Vec<String> = if search_terms_input.trim().is_empty() {
        Vec::new()
    } else {
        search_terms_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    // Optional notes
    let notes = ui::prompt_text_with_default("Notes (optional):", "")?;
    let notes = if notes.is_empty() { None } else { Some(notes) };

    // Generate key
    let key = config.generate_unique_key(&name);

    // Create subject
    let subject = Subject {
        id: Uuid::new_v4(),
        key,
        name: name.clone(),
        subject_type,
        category,
        question,
        event_name,
        search_terms,
        notes,
        enabled: true,
    };

    // Validate
    subject.validate().map_err(|e| HeadsupError::Config(e))?;

    // Add to config
    config.subjects.push(subject);
    config::save_config(config)?;

    ui::print_success(&format!("Added '{}' to your headsup", name));

    Ok(())
}

fn remove_subject(key: &str) -> Result<()> {
    let mut config = config::load_config()?;

    let idx = config.subjects.iter()
        .position(|s| s.key.eq_ignore_ascii_case(key) || s.id.to_string() == key)
        .ok_or_else(|| HeadsupError::SubjectNotFound(key.to_string()))?;

    let subject = config.subjects.remove(idx);
    config::save_config(&config)?;

    ui::print_success(&format!("Removed '{}'", subject.name));

    Ok(())
}

fn edit_subject(key: &str) -> Result<()> {
    if !ui::is_interactive() {
        return Err(HeadsupError::Config(
            "Interactive mode required. Edit config file directly.".to_string(),
        ));
    }

    let mut config = config::load_config()?;

    let subject = config.find_subject_mut(key)
        .ok_or_else(|| HeadsupError::SubjectNotFound(key.to_string()))?;

    // Edit key
    let current_key = subject.key.clone();
    let new_key = ui::prompt_text_with_default("Key:", &current_key)?;

    // Validate new key if changed
    if new_key != current_key {
        // Check for conflicts (need to temporarily release the borrow)
        let new_key_lower = new_key.to_lowercase();
        let conflict = config.subjects.iter()
            .any(|s| s.key.to_lowercase() == new_key_lower && s.key != current_key);
        if conflict {
            return Err(HeadsupError::SubjectKeyExists(new_key));
        }
    }

    // Re-borrow after conflict check
    let subject = config.find_subject_mut(key).unwrap();
    subject.key = new_key;

    // Edit name
    let name = ui::prompt_text_with_default("Name:", &subject.name)?;
    subject.name = name;

    // Edit search terms (optional - AI can determine queries from context)
    let current_terms = subject.search_terms.join(", ");
    let new_terms = ui::prompt_text_with_default("Search terms (comma-separated, or leave empty):", &current_terms)?;
    subject.search_terms = if new_terms.trim().is_empty() {
        Vec::new()
    } else {
        new_terms
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    // Edit notes
    let current_notes = subject.notes.clone().unwrap_or_default();
    let new_notes = ui::prompt_text_with_default("Notes:", &current_notes)?;
    subject.notes = if new_notes.is_empty() { None } else { Some(new_notes) };

    // Validate
    subject.validate().map_err(|e| HeadsupError::Config(e))?;

    config::save_config(&config)?;
    ui::print_success("Subject updated");

    Ok(())
}

fn enable_subject(key: &str) -> Result<()> {
    let mut config = config::load_config()?;

    let subject = config.find_subject_mut(key)
        .ok_or_else(|| HeadsupError::SubjectNotFound(key.to_string()))?;

    if subject.enabled {
        ui::print_info(&format!("'{}' is already enabled", subject.name));
    } else {
        subject.enabled = true;
        let name = subject.name.clone();
        config::save_config(&config)?;
        ui::print_success(&format!("Enabled '{}'", name));
    }

    Ok(())
}

fn disable_subject(key: &str) -> Result<()> {
    let mut config = config::load_config()?;

    let subject = config.find_subject_mut(key)
        .ok_or_else(|| HeadsupError::SubjectNotFound(key.to_string()))?;

    if !subject.enabled {
        ui::print_info(&format!("'{}' is already disabled", subject.name));
    } else {
        subject.enabled = false;
        let name = subject.name.clone();
        config::save_config(&config)?;
        ui::print_success(&format!("Disabled '{}'", name));
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
