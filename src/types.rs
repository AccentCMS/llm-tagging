//! Shared request/response types for the LLM tagging plugin.
//!
//! These types define the JSON wire format used by the plugin's route handlers
//! (the body of each `/api/llm/*` request and response) and the structured data
//! returned by LLM providers. The hook/route/filter *envelope* types the Extism
//! version carried are gone: on the Component Model the host hands the guest
//! typed WIT records (`routes::request`, `filters::filter-input`), so the plugin
//! no longer hand-deserializes a JSON envelope.

use serde::{Deserialize, Serialize};

/// Request body for `POST /api/llm/analyze`.
///
/// Triggers LLM analysis for a single page. The `content` field carries the
/// raw markdown body so the WASM plugin does not need filesystem access.
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    /// URL path of the page to analyze (e.g., `/blog/my-post`).
    pub page_path: String,

    /// Page title (from frontmatter).
    #[serde(default)]
    pub title: String,

    /// Raw markdown content of the page.
    #[serde(default)]
    pub content: String,

    /// When true, re-analyze even if tags/lead already exist (reserved for Phase 2 caching).
    #[serde(default)]
    #[allow(dead_code)]
    pub force: bool,

    /// Which analysis types to run. Defaults to both tags and lead.
    #[serde(default = "default_analysis_types")]
    pub types: Vec<String>,
}

fn default_analysis_types() -> Vec<String> {
    vec!["tags".to_string(), "lead".to_string()]
}

/// Response body for `POST /api/llm/analyze`.
#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    /// Generated tags (empty if tag analysis was not requested).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Generated lead/summary text (empty if lead analysis was not requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,

    /// LLM confidence score for the analysis (0.0-1.0).
    pub confidence: f64,

    /// Model identifier used for analysis.
    pub model: String,

    /// The page path that was analyzed.
    pub page_path: String,
}

/// Request body for `POST /api/llm/batch`.
///
/// Triggers LLM analysis for multiple pages sequentially.
#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    /// List of pages to analyze, each with path, title, and content.
    pub pages: Vec<BatchPageInput>,

    /// When true, re-analyze even if tags/lead already exist (reserved for Phase 2 caching).
    #[serde(default)]
    #[allow(dead_code)]
    pub force: bool,

    /// Which analysis types to run.
    #[serde(default = "default_analysis_types")]
    pub types: Vec<String>,
}

/// A single page entry in a batch request.
#[derive(Debug, Deserialize)]
pub struct BatchPageInput {
    /// URL path of the page.
    pub page_path: String,

    /// Page title.
    #[serde(default)]
    pub title: String,

    /// Raw markdown content.
    #[serde(default)]
    pub content: String,
}

/// Response body for `POST /api/llm/batch`.
#[derive(Debug, Serialize)]
pub struct BatchResponse {
    /// Results for each page (in the same order as the request).
    pub results: Vec<PageResult>,

    /// Total number of pages processed.
    pub total: usize,
}

/// Result for a single page in a batch response.
#[derive(Debug, Serialize)]
pub struct PageResult {
    /// The page path.
    pub page_path: String,

    /// Whether analysis succeeded.
    pub success: bool,

    /// Analysis result (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Lead text (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,

    /// Error message (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response body for `GET /api/llm/taxonomy`.
#[derive(Debug, Serialize)]
pub struct TaxonomyResponse {
    /// Suggested tags derived from configuration's preferred_tags list.
    pub tags: Vec<TagSuggestion>,
}

/// A single tag suggestion with usage metadata.
#[derive(Debug, Serialize)]
pub struct TagSuggestion {
    /// The tag value.
    pub tag: String,

    /// Whether this tag is in the preferred list.
    pub preferred: bool,
}

/// Parsed LLM response for tag generation.
#[derive(Debug, Deserialize)]
pub struct LlmTagResponse {
    /// Generated tags.
    pub tags: Vec<String>,

    /// Optional reasoning from the LLM (present in JSON but not always read).
    #[serde(default)]
    #[allow(dead_code)]
    pub reasoning: String,
}

/// Parsed LLM response for lead generation.
#[derive(Debug, Deserialize)]
pub struct LlmLeadResponse {
    /// Generated lead text.
    pub lead: String,

    /// Optional alternative leads (present in JSON but not always read).
    #[serde(default)]
    #[allow(dead_code)]
    pub alternatives: Vec<String>,
}
