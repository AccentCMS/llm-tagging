//! Plugin configuration parsing for the LLM tagging plugin (Feature f026a).
//!
//! Configuration is passed from the Accent CMS host as a JSON blob derived
//! from the `[config]` section of `plugin.toml`. This module deserializes
//! it into strongly-typed structures.

use serde::Deserialize;

/// Top-level plugin configuration.
#[derive(Debug, Default, Deserialize)]
pub struct PluginConfig {
    /// LLM provider settings.
    #[serde(default)]
    pub provider: ProviderConfig,

    /// Tag generation settings.
    #[serde(default)]
    pub tags: TagsConfig,

    /// Lead generation settings.
    #[serde(default)]
    pub lead: LeadConfig,

    /// Workflow settings (auto-analyze behavior). Reserved: it drove the
    /// `on_page_load` auto-analysis the Extism version carried, which is not
    /// exported on the component contract because synchronous LLM calls would
    /// block the render path. It re-activates when page-load hooks can run
    /// async (E034 post-parity); kept so `[config.workflow]` still parses.
    #[serde(default)]
    #[allow(dead_code)]
    pub workflow: WorkflowConfig,
}

/// LLM provider configuration.
#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    /// Provider type: "anthropic", "openai", or "ollama".
    #[serde(default = "default_provider_type", rename = "type")]
    pub provider_type: String,

    /// Environment variable name containing the API key.
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Model identifier.
    #[serde(default = "default_model")]
    pub model: String,

    /// Base URL override for custom/self-hosted endpoints.
    #[serde(default)]
    pub base_url: String,

    /// Request timeout in seconds (reserved for future use with async providers).
    #[serde(default = "default_timeout")]
    #[allow(dead_code)]
    pub timeout_seconds: u64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: default_provider_type(),
            api_key_env: default_api_key_env(),
            model: default_model(),
            base_url: String::new(),
            timeout_seconds: default_timeout(),
        }
    }
}

fn default_provider_type() -> String {
    "anthropic".to_string()
}

fn default_api_key_env() -> String {
    "LLM_API_KEY".to_string()
}

fn default_model() -> String {
    "claude-3-haiku-20240307".to_string()
}

fn default_timeout() -> u64 {
    30
}

/// Tag generation configuration.
#[derive(Debug, Deserialize)]
pub struct TagsConfig {
    /// Whether tag generation is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum number of tags to generate per page.
    #[serde(default = "default_max_tags")]
    pub max_tags: usize,

    /// Minimum number of tags to generate per page.
    #[serde(default = "default_min_tags")]
    pub min_tags: usize,

    /// Tag formatting style: "lowercase-hyphen", "lowercase", "title-case".
    #[serde(default = "default_tag_style")]
    pub style: String,

    /// Preferred tags for consistency across the site.
    #[serde(default)]
    pub preferred_tags: Vec<String>,

    /// Tags that should never be generated.
    #[serde(default)]
    pub blocked_tags: Vec<String>,
}

impl Default for TagsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tags: default_max_tags(),
            min_tags: default_min_tags(),
            style: default_tag_style(),
            preferred_tags: Vec::new(),
            blocked_tags: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_tags() -> usize {
    8
}

fn default_min_tags() -> usize {
    3
}

fn default_tag_style() -> String {
    "lowercase-hyphen".to_string()
}

/// Lead/summary generation configuration.
#[derive(Debug, Deserialize)]
pub struct LeadConfig {
    /// Whether lead generation is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum character length for generated leads.
    #[serde(default = "default_max_lead_length")]
    pub max_length: usize,

    /// Minimum character length for generated leads.
    #[serde(default = "default_min_lead_length")]
    pub min_length: usize,

    /// Writing style: "informative", "engaging", "technical", "casual".
    #[serde(default = "default_lead_style")]
    pub style: String,
}

impl Default for LeadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_length: default_max_lead_length(),
            min_length: default_min_lead_length(),
            style: default_lead_style(),
        }
    }
}

fn default_max_lead_length() -> usize {
    200
}

fn default_min_lead_length() -> usize {
    50
}

fn default_lead_style() -> String {
    "informative".to_string()
}

/// Workflow configuration controlling auto-analysis behavior. Reserved for the
/// post-parity async page-load hook (see [`PluginConfig::workflow`]); parsed
/// from `[config.workflow]` but not read while the content hook is absent.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WorkflowConfig {
    /// Automatically mark pages for analysis on page load.
    #[serde(default)]
    pub auto_analyze: bool,

    /// Skip analysis if tags/lead already exist in frontmatter.
    #[serde(default = "default_true")]
    pub skip_if_exists: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            auto_analyze: false,
            skip_if_exists: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PluginConfig::default();
        assert_eq!(config.provider.provider_type, "anthropic");
        assert_eq!(config.provider.api_key_env, "LLM_API_KEY");
        assert!(config.tags.enabled);
        assert_eq!(config.tags.max_tags, 8);
        assert!(config.lead.enabled);
        assert!(!config.workflow.auto_analyze);
    }

    #[test]
    fn test_deserialize_config() {
        let json = r#"{
            "provider": {
                "type": "openai",
                "api_key_env": "OPENAI_KEY",
                "model": "gpt-4"
            },
            "tags": {
                "enabled": true,
                "max_tags": 5,
                "preferred_tags": ["rust", "web"]
            }
        }"#;
        let config: PluginConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.provider.provider_type, "openai");
        assert_eq!(config.provider.model, "gpt-4");
        assert_eq!(config.tags.max_tags, 5);
        assert_eq!(config.tags.preferred_tags, vec!["rust", "web"]);
    }
}
