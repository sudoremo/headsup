use crate::cli::StateCommands;
use crate::config;
use crate::error::{HeadsupError, Result};
use crate::state;
use crate::ui;

/// Run state subcommands
pub fn run_state(command: StateCommands) -> Result<()> {
    match command {
        StateCommands::Show => show_state(),
        StateCommands::Prune => prune_state(),
        StateCommands::Reset { key } => reset_state(key),
        StateCommands::Path => print_path(),
    }
}

fn show_state() -> Result<()> {
    let state = state::load_state_readonly()?;
    let content = serde_json::to_string_pretty(&state)
        .map_err(|e| HeadsupError::State(format!("Failed to serialize state: {}", e)))?;
    println!("{}", content);
    Ok(())
}

fn prune_state() -> Result<()> {
    let config = config::load_config()?;
    let (mut state, lock) = state::load_state()?;

    // Get list of valid subject IDs from config
    let valid_ids: Vec<_> = config.subjects.iter().map(|s| s.id).collect();

    // Prune orphans
    let orphans = state.prune_orphans(&valid_ids);

    if orphans.is_empty() {
        ui::print_info("No orphaned state entries found");
    } else {
        state::save_state(&state, &lock)?;
        ui::print_success(&format!("Pruned {} orphaned state entries", orphans.len()));
        for id in orphans {
            ui::print_info(&format!("  Removed: {}", id));
        }
    }

    Ok(())
}

fn reset_state(key: Option<String>) -> Result<()> {
    let (mut state, lock) = state::load_state()?;

    match key {
        Some(key_or_id) => {
            // Reset specific subject
            let config = config::load_config()?;
            let subject = config.find_subject(&key_or_id)
                .ok_or_else(|| HeadsupError::SubjectNotFound(key_or_id.clone()))?;

            if state.subjects.remove(&subject.id).is_some() {
                state::save_state(&state, &lock)?;
                ui::print_success(&format!("Reset state for '{}'", subject.name));
            } else {
                ui::print_info(&format!("No state found for '{}'", subject.name));
            }
        }
        None => {
            // Reset all state
            if ui::is_interactive() {
                let confirm = ui::prompt_confirm("Reset all state? This cannot be undone.", false)?;
                if !confirm {
                    ui::print_info("Cancelled");
                    return Ok(());
                }
            }

            let count = state.subjects.len();
            state.subjects.clear();
            state.pending_notifications.clear();
            state::save_state(&state, &lock)?;
            ui::print_success(&format!("Reset state for {} subjects", count));
        }
    }

    Ok(())
}

fn print_path() -> Result<()> {
    let path = config::state_path()?;
    println!("{}", path.display());
    Ok(())
}
