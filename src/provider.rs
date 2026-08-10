//! LLM provider abstraction for the tagging plugin.
//!
//! Makes outbound HTTP calls to LLM APIs through the host's `outbound-http`
//! capability (see [`crate::host`]). Supports three provider backends:
//!
//! - **Anthropic** (Messages API) - Claude models
//! - **OpenAI** (Chat Completions API) - GPT models
//! - **Ollama** (local) - self-hosted open-source models

use serde_json::json;

use crate::config::ProviderConfig;
use crate::host::{config_get, http_post_json};

/// Call an LLM provider with the given prompt and return the response text.
///
/// Dispatches to the appropriate provider based on `config.provider_type`.
/// The API key is read from the merged plugin config (injected from the
/// environment variable named in `config.api_key_env`).
///
/// # Errors
///
/// Returns an error string if the provider type is unknown, the API key is
/// missing or empty, the HTTP request fails, or the response cannot be parsed.
pub fn call_llm(config: &ProviderConfig, prompt: &str) -> Result<String, String> {
    let api_key = config_get(&config.api_key_env).unwrap_or_default();

    match config.provider_type.as_str() {
        "anthropic" => call_anthropic(config, &api_key, prompt),
        "openai" => call_openai(config, &api_key, prompt),
        "ollama" => call_ollama(config, prompt),
        other => Err(format!("unknown LLM provider: {other}")),
    }
}

/// Call the Anthropic Messages API.
fn call_anthropic(config: &ProviderConfig, api_key: &str, prompt: &str) -> Result<String, String> {
    if api_key.is_empty() {
        return Err(format!(
            "Anthropic API key not configured (set {} env var)",
            config.api_key_env
        ));
    }

    let base_url = if config.base_url.is_empty() {
        "https://api.anthropic.com"
    } else {
        &config.base_url
    };
    let url = format!("{base_url}/v1/messages");

    let body = json!({
        "model": config.model,
        "max_tokens": 1024,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let (status, response_body) = http_post_json(
        &url,
        &[
            ("x-api-key", api_key),
            ("anthropic-version", "2023-06-01"),
            ("content-type", "application/json"),
        ],
        body.to_string(),
    )
    .map_err(|e| format!("Anthropic HTTP request failed: {e}"))?;

    if status != 200 {
        return Err(format!(
            "Anthropic API error (HTTP {status}): {response_body}"
        ));
    }

    // Parse Anthropic response: extract text from content[0].text
    let parsed: serde_json::Value = serde_json::from_str(&response_body)
        .map_err(|e| format!("failed to parse Anthropic response: {e}"))?;

    parsed["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Anthropic response missing content[0].text".to_string())
}

/// Call the OpenAI Chat Completions API.
fn call_openai(config: &ProviderConfig, api_key: &str, prompt: &str) -> Result<String, String> {
    if api_key.is_empty() {
        return Err(format!(
            "OpenAI API key not configured (set {} env var)",
            config.api_key_env
        ));
    }

    let base_url = if config.base_url.is_empty() {
        "https://api.openai.com"
    } else {
        &config.base_url
    };
    let url = format!("{base_url}/v1/chat/completions");

    let body = json!({
        "model": config.model,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.3
    });

    let authorization = format!("Bearer {api_key}");
    let (status, response_body) = http_post_json(
        &url,
        &[
            ("Authorization", authorization.as_str()),
            ("content-type", "application/json"),
        ],
        body.to_string(),
    )
    .map_err(|e| format!("OpenAI HTTP request failed: {e}"))?;

    if status != 200 {
        return Err(format!("OpenAI API error (HTTP {status}): {response_body}"));
    }

    // Parse OpenAI response: extract text from choices[0].message.content
    let parsed: serde_json::Value = serde_json::from_str(&response_body)
        .map_err(|e| format!("failed to parse OpenAI response: {e}"))?;

    parsed["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "OpenAI response missing choices[0].message.content".to_string())
}

/// Call a local Ollama instance.
fn call_ollama(config: &ProviderConfig, prompt: &str) -> Result<String, String> {
    let base_url = if config.base_url.is_empty() {
        "http://localhost:11434"
    } else {
        &config.base_url
    };
    let url = format!("{base_url}/api/generate");

    let body = json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.3
        }
    });

    let (status, response_body) = http_post_json(
        &url,
        &[("content-type", "application/json")],
        body.to_string(),
    )
    .map_err(|e| format!("Ollama HTTP request failed: {e}"))?;

    if status != 200 {
        return Err(format!("Ollama API error (HTTP {status}): {response_body}"));
    }

    // Parse Ollama response: extract text from response field
    let parsed: serde_json::Value = serde_json::from_str(&response_body)
        .map_err(|e| format!("failed to parse Ollama response: {e}"))?;

    parsed["response"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Ollama response missing 'response' field".to_string())
}
