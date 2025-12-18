use crate::config::ClaudeConfig;
use crate::error::{HeadsupError, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

/// Execute a Claude query with the given prompt
pub async fn execute_claude(config: &ClaudeConfig, prompt: &str) -> Result<String> {
    let timeout_duration = Duration::from_secs(config.timeout_seconds);

    // Run Claude in a blocking task with timeout
    let prompt_owned = prompt.to_string();
    let command = config.command.clone();
    let model = config.model.clone();

    let result = timeout(timeout_duration, async move {
        tokio::task::spawn_blocking(move || {
            execute_claude_sync(&command, &model, &prompt_owned)
        })
        .await
        .map_err(|e| HeadsupError::Claude(format!("Task join error: {}", e)))?
    })
    .await;

    match result {
        Ok(inner_result) => inner_result,
        Err(_) => Err(HeadsupError::ClaudeTimeout(config.timeout_seconds)),
    }
}

/// Execute Claude synchronously
fn execute_claude_sync(command: &str, model: &str, prompt: &str) -> Result<String> {
    // Build the command
    // The command might be a simple "claude" or a full path or include arguments
    let (program, base_args) = parse_command(command);

    let mut cmd = Command::new(&program);
    cmd.args(&base_args)
        .arg("--print")
        .arg("--model")
        .arg(model)
        .arg("--allowedTools")
        .arg("WebSearch")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| HeadsupError::Claude(format!("Failed to spawn Claude process: {}", e)))?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())
            .map_err(|e| HeadsupError::Claude(format!("Failed to write to Claude stdin: {}", e)))?;
    }

    // Wait for completion
    let output = child.wait_with_output()
        .map_err(|e| HeadsupError::Claude(format!("Failed to wait for Claude: {}", e)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stdout.trim().is_empty() {
            Err(HeadsupError::Claude("Claude returned empty response".to_string()))
        } else {
            Ok(stdout)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(HeadsupError::Claude(format!(
            "Claude exited with status {}: {}",
            output.status,
            stderr.trim()
        )))
    }
}

/// Parse a command string into program and arguments
/// Handles cases like:
/// - "claude"
/// - "/usr/local/bin/claude"
/// - "claude --profile work"
fn parse_command(command: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        ("claude".to_string(), vec![])
    } else {
        (parts[0].to_string(), parts[1..].iter().map(|s| s.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_simple() {
        let (program, args) = parse_command("claude");
        assert_eq!(program, "claude");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_command_with_path() {
        let (program, args) = parse_command("/usr/local/bin/claude");
        assert_eq!(program, "/usr/local/bin/claude");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_command_with_args() {
        let (program, args) = parse_command("claude --profile work");
        assert_eq!(program, "claude");
        assert_eq!(args, vec!["--profile", "work"]);
    }
}
