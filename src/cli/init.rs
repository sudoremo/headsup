use crate::config::{self, Config};
use crate::error::Result;
use crate::ui;

/// Run the init command
pub fn run_init(force: bool, email: Option<String>) -> Result<()> {
    let config_path = config::config_path()?;

    // Check if config already exists
    if config_path.exists() && !force {
        ui::print_warning(&format!(
            "Config file already exists at {}",
            config_path.display()
        ));
        ui::print_info("Use --force to overwrite");
        return Ok(());
    }

    // Get email address
    let email_addr = if let Some(e) = email {
        e
    } else if ui::is_interactive() {
        ui::prompt_text("Email address for notifications:")?
    } else {
        return Err(crate::error::HeadsupError::Config(
            "Email address required (use --email flag)".to_string(),
        ));
    };

    // Validate email (basic check)
    if !email_addr.contains('@') {
        return Err(crate::error::HeadsupError::Config(
            "Invalid email address format".to_string(),
        ));
    }

    // Create default config
    let config = Config::default_with_email(&email_addr);

    // Save config
    config::save_config(&config)?;

    ui::print_success(&format!(
        "Created config file at {}",
        config_path.display()
    ));
    ui::print_info("Edit the config file to configure your SMTP settings");

    Ok(())
}
