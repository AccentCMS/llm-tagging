//! LLM-powered document tagging and lead generation plugin, on the WebAssembly
//! Component Model.
//!
//! Implements the `networked-api-plugin` world:
//!
//! - `routes::handle` dispatches `POST /api/llm/analyze`, `POST /api/llm/batch`,
//!   and `GET /api/llm/taxonomy`. Analysis routes call the configured LLM
//!   provider (Anthropic / OpenAI / Ollama) through the `outbound-http`
//!   capability.
//! - `filters::apply` provides the `format_tags` template filter.
//!
//! # Design Decisions
//!
//! - LLM calls take 2-30 seconds and the sync render path would block, so the
//!   plugin does NOT register a content hook; analysis is triggered via the
//!   route handlers. (The Extism version exported a disabled `on_page_load`
//!   pass-through; on the typed contract a disabled hook is simply not part of
//!   the world.)
//! - Outbound HTTP is granted structurally: the world imports `outbound-http`
//!   and the manifest declares `allowed_hosts`.

#[allow(warnings)]
mod bindings;

mod config;
mod host;
mod prompts;
mod provider;
mod types;

use serde_json::json;

use config::PluginConfig;
use types::*;

use bindings::accent::plugin::types::Json;
use bindings::exports::accent::plugin::filters::{FilterInput, Guest as Filters};
use bindings::exports::accent::plugin::routes::{Guest as Routes, Request, Response};

// Feature f210 (Component Model port of the f026a llm-tagging plugin).
/// The unit struct the host instantiates. Its `Guest` impls are the plugin.
struct Component;

// ---------------------------------------------------------------------------
// routes
// ---------------------------------------------------------------------------

impl Routes for Component {
    /// Dispatch by `(method, path)`. Every outcome (success or handler error)
    /// is returned as a 200 response whose JSON body carries either the result
    /// payload or `{ "error": ..., "success": false }`, matching the Extism
    /// version's behaviour of always returning the payload string.
    fn handle(req: Request) -> Result<Response, String> {
        let result = match (req.method.as_str(), req.path.as_str()) {
            ("POST", "/api/llm/analyze") => handle_analyze(&req.body),
            ("POST", "/api/llm/batch") => handle_batch(&req.body),
            ("GET", "/api/llm/taxonomy") => handle_taxonomy(),
            _ => Err(format!("unknown route: {} {}", req.method, req.path)),
        };

        let body = match result {
            Ok(response) => response,
            Err(err) => json!({ "error": err, "success": false }).to_string(),
        };

        Ok(Response {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body,
        })
    }
}

/// Handle `POST /api/llm/analyze` - analyze a single page.
fn handle_analyze(body: &str) -> Result<String, String> {
    let req: AnalyzeRequest =
        serde_json::from_str(body).map_err(|e| format!("invalid analyze request: {e}"))?;

    if req.content.is_empty() {
        return Err("content is required for analysis".to_string());
    }

    let config = load_config()?;
    let mut tags = Vec::new();
    let mut lead = None;
    let mut confidence = 0.0;

    if req.types.iter().any(|t| t == "tags") && config.tags.enabled {
        let prompt =
            prompts::build_tag_prompt(&req.title, &req.page_path, &req.content, &config.tags);
        match provider::call_llm(&config.provider, &prompt) {
            Ok(response_text) => match parse_tag_response(&response_text) {
                Ok(tag_result) => {
                    tags = filter_tags(tag_result.tags, &config.tags);
                    confidence = 0.9;
                }
                Err(e) => return Err(format!("failed to parse tag response: {e}")),
            },
            Err(e) => return Err(format!("LLM tag generation failed: {e}")),
        }
    }

    if req.types.iter().any(|t| t == "lead") && config.lead.enabled {
        let prompt =
            prompts::build_lead_prompt(&req.title, &req.page_path, &req.content, &config.lead);
        match provider::call_llm(&config.provider, &prompt) {
            Ok(response_text) => match parse_lead_response(&response_text) {
                Ok(lead_result) => {
                    lead = Some(lead_result.lead);
                    confidence = if confidence > 0.0 {
                        (confidence + 0.9) / 2.0
                    } else {
                        0.9
                    };
                }
                Err(e) => return Err(format!("failed to parse lead response: {e}")),
            },
            Err(e) => return Err(format!("LLM lead generation failed: {e}")),
        }
    }

    let response = AnalyzeResponse {
        tags,
        lead,
        confidence,
        model: config.provider.model.clone(),
        page_path: req.page_path,
    };

    serde_json::to_string(&response).map_err(|e| format!("failed to serialize response: {e}"))
}

/// Handle `POST /api/llm/batch` - analyze multiple pages sequentially.
fn handle_batch(body: &str) -> Result<String, String> {
    let req: BatchRequest =
        serde_json::from_str(body).map_err(|e| format!("invalid batch request: {e}"))?;

    let config = load_config()?;
    let mut results = Vec::with_capacity(req.pages.len());

    for page in &req.pages {
        if page.content.is_empty() {
            results.push(PageResult {
                page_path: page.page_path.clone(),
                success: false,
                tags: None,
                lead: None,
                error: Some("empty content".to_string()),
            });
            continue;
        }

        let mut page_tags = None;
        let mut page_lead = None;
        let mut error = None;

        if req.types.iter().any(|t| t == "tags") && config.tags.enabled {
            let prompt = prompts::build_tag_prompt(
                &page.title,
                &page.page_path,
                &page.content,
                &config.tags,
            );
            match provider::call_llm(&config.provider, &prompt) {
                Ok(text) => match parse_tag_response(&text) {
                    Ok(tag_result) => {
                        page_tags = Some(filter_tags(tag_result.tags, &config.tags));
                    }
                    Err(e) => error = Some(format!("tag parse error: {e}")),
                },
                Err(e) => error = Some(format!("tag generation error: {e}")),
            }
        }

        if error.is_none() && req.types.iter().any(|t| t == "lead") && config.lead.enabled {
            let prompt = prompts::build_lead_prompt(
                &page.title,
                &page.page_path,
                &page.content,
                &config.lead,
            );
            match provider::call_llm(&config.provider, &prompt) {
                Ok(text) => match parse_lead_response(&text) {
                    Ok(lead_result) => {
                        page_lead = Some(lead_result.lead);
                    }
                    Err(e) => error = Some(format!("lead parse error: {e}")),
                },
                Err(e) => error = Some(format!("lead generation error: {e}")),
            }
        }

        results.push(PageResult {
            page_path: page.page_path.clone(),
            success: error.is_none(),
            tags: page_tags,
            lead: page_lead,
            error,
        });
    }

    let total = results.len();
    let response = BatchResponse { results, total };
    serde_json::to_string(&response).map_err(|e| format!("failed to serialize response: {e}"))
}

/// Handle `GET /api/llm/taxonomy` - return preferred tags as suggestions.
fn handle_taxonomy() -> Result<String, String> {
    let config = load_config()?;

    let tags: Vec<TagSuggestion> = config
        .tags
        .preferred_tags
        .iter()
        .map(|tag| TagSuggestion {
            tag: tag.clone(),
            preferred: true,
        })
        .collect();

    let response = TaxonomyResponse { tags };
    serde_json::to_string(&response).map_err(|e| format!("failed to serialize response: {e}"))
}

// ---------------------------------------------------------------------------
// filters
// ---------------------------------------------------------------------------

impl Filters for Component {
    /// The host dispatches by `filter-name`; this plugin owns `format_tags`.
    fn apply(input: FilterInput) -> Result<Json, String> {
        match input.filter_name.as_str() {
            "format_tags" => format_tags(&input.value, &input.args),
            other => Err(format!("unknown filter: {other}")),
        }
    }
}

/// Format a tags array for display: join an array of strings (or pass a string
/// through unchanged). Default separator is ", "; an optional first argument
/// overrides it. The `json` payloads are UTF-8 JSON documents (plugin API
/// 0.2.0), read as `serde_json::Value` and answered as a JSON string.
fn format_tags(value: &[u8], args: &[Json]) -> Result<Json, String> {
    let value = decode_json(value)?;
    let separator = match args.first() {
        Some(arg) => decode_json(arg)?
            .as_str()
            .map_or_else(|| ", ".to_string(), str::to_owned),
        None => ", ".to_string(),
    };

    let formatted = match value {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>()
            .join(&separator),
        serde_json::Value::String(s) => s,
        _ => String::new(),
    };

    serde_json::to_vec(&serde_json::Value::String(formatted))
        .map_err(|e| format!("failed to encode filter result: {e}"))
}

/// Decode a `json` payload. The empty payload is the contract's "no value"
/// and reads as `null`; anything else must be one JSON document.
fn decode_json(bytes: &[u8]) -> Result<serde_json::Value, String> {
    if bytes.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_slice(bytes).map_err(|e| format!("filter input is not valid JSON: {e}"))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load plugin configuration from the merged host config.
///
/// As under Extism, the nested `[config]` sections are not surfaced under a
/// single `config` key, so this resolves to `PluginConfig::default()` unless a
/// host explicitly provides a `config` JSON blob.
fn load_config() -> Result<PluginConfig, String> {
    let config_json = host::config_get("config").unwrap_or_else(|| "{}".to_string());
    serde_json::from_str(&config_json).map_err(|e| format!("failed to parse plugin config: {e}"))
}

/// Parse an LLM response as tag generation output, tolerating markdown fences.
fn parse_tag_response(text: &str) -> Result<LlmTagResponse, String> {
    let json_text = extract_json(text);
    serde_json::from_str(json_text).map_err(|e| format!("invalid tag JSON: {e}"))
}

/// Parse an LLM response as lead generation output.
fn parse_lead_response(text: &str) -> Result<LlmLeadResponse, String> {
    let json_text = extract_json(text);
    serde_json::from_str(json_text).map_err(|e| format!("invalid lead JSON: {e}"))
}

/// Extract JSON from LLM response text, stripping markdown code fences.
fn extract_json(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return &trimmed[start..=end];
        }
    }
    trimmed
}

/// Filter generated tags: remove blocked tags and enforce the max count.
fn filter_tags(mut tags: Vec<String>, config: &config::TagsConfig) -> Vec<String> {
    tags.retain(|t| {
        !config
            .blocked_tags
            .iter()
            .any(|blocked| blocked.eq_ignore_ascii_case(t))
    });
    tags.truncate(config.max_tags);
    tags
}

// Wires `Component` into the generated component exports. Required by
// cargo-component; keep it as the last item in the crate.
bindings::export!(Component with_types_in bindings);
