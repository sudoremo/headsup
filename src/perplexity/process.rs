use crate::config::PerplexityConfig;
use crate::error::{HeadsupError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

const PERPLEXITY_API_URL: &str = "https://api.perplexity.ai/chat/completions";

#[derive(Debug, Serialize)]
struct PerplexityRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct PerplexityResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Execute a Perplexity API query with the given prompt
pub async fn execute_perplexity(config: &PerplexityConfig, prompt: &str) -> Result<String> {
    let timeout_duration = Duration::from_secs(config.timeout_seconds);

    // Get API key from command
    let api_key = get_api_key(&config.api_key_command)?;

    let client = Client::builder()
        .timeout(timeout_duration)
        .build()
        .map_err(|e| HeadsupError::Perplexity(format!("Failed to create HTTP client: {}", e)))?;

    let request = PerplexityRequest {
        model: config.model.clone(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post(PERPLEXITY_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                HeadsupError::PerplexityTimeout(config.timeout_seconds)
            } else {
                HeadsupError::Perplexity(format!("Request failed: {}", e))
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(HeadsupError::Perplexity(format!(
            "API returned status {}: {}",
            status, body
        )));
    }

    let perplexity_response: PerplexityResponse = response
        .json()
        .await
        .map_err(|e| HeadsupError::Perplexity(format!("Failed to parse response: {}", e)))?;

    let content = perplexity_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| HeadsupError::Perplexity("No response content".to_string()))?;

    if content.trim().is_empty() {
        return Err(HeadsupError::Perplexity("Empty response".to_string()));
    }

    Ok(content)
}

/// Get API key by executing the configured command
fn get_api_key(command: &str) -> Result<String> {
    if command.is_empty() {
        return Err(HeadsupError::Perplexity(
            "Perplexity API key command not configured".to_string(),
        ));
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| HeadsupError::Perplexity(format!("Failed to execute API key command: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HeadsupError::Perplexity(format!(
            "API key command failed: {}",
            stderr.trim()
        )));
    }

    let api_key = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if api_key.is_empty() {
        return Err(HeadsupError::Perplexity(
            "API key command returned empty result".to_string(),
        ));
    }

    Ok(api_key)
}
