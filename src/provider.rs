//! LLM provider abstraction for the tagging plugin.
//!
//! Supports four provider backends:
//!
//! - **Host** - the operator's model, which the host calls through the
//!   `host-services` capability (`plugins.model` in config.yaml). Use it for a
//!   model on the same machine or the LAN, such as Ollama on
//!   `localhost:11434`.
//! - **Anthropic** (Messages API) - Claude models, over `outbound-http`
//! - **OpenAI** (Chat Completions API) - GPT models, over `outbound-http`
//! - **Ollama** - an Ollama server on a public address, over `outbound-http`
//!
//! See [`crate::host`] for the capability wrappers.

use serde_json::json;

use crate::config::ProviderConfig;
use crate::host::{config_get, host_model_complete, http_post_json};

/// The answer limit the plugin asks for. The host clamps it to the
/// operator's `plugins.model.max_tokens`.
const MAX_ANSWER_TOKENS: u32 = 1024;

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
    // The host provider has no key: the host holds it.
    if config.provider_type == "host" {
        return host_model_complete(prompt, MAX_ANSWER_TOKENS);
    }
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

/// Call an Ollama server.
///
/// `base_url` has no default. This provider calls the server over
/// `outbound-http`, which refuses every loopback and private address, so the
/// server must be on a public address that `allowed_hosts` in `plugin.toml`
/// lists. For Ollama on `localhost:11434` or the LAN, use the `host` provider
/// and point `plugins.model` at Ollama's OpenAI-compatible endpoint.
///
/// # Errors
///
/// Returns an error string when `base_url` is empty, the request fails, or
/// the response cannot be parsed.
fn call_ollama(config: &ProviderConfig, prompt: &str) -> Result<String, String> {
    if config.base_url.is_empty() {
        return Err(
            "Ollama needs `base_url` set to a public address that the plugin's \
             `allowed_hosts` lists. For Ollama on this machine or the LAN, set \
             `type = \"host\"` and configure `plugins.model` in config.yaml"
                .to_string(),
        );
    }
    let url = format!("{}/api/generate", config.base_url);

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
