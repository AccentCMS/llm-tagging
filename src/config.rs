//! Plugin configuration parsing for the LLM tagging plugin (Feature f026a).
//!
//! The host merges the `[config.<section>]` tables of `plugin.toml` with the
//! `plugins.config.llm-tagging` overrides in `config.yaml`, and serves each
//! section as one key whose value is an inline TOML table, for example
//! `provider` = `{ type = "host", model = "m" }`. [`from_sections`] reads
//! those keys back into strongly-typed structures.

use serde::de::DeserializeOwned;
use serde::Deserialize;

/// Build the configuration from the per-section values that `get` returns.
///
/// Each section is read on its own, so an error names the section that is
/// wrong. A missing section takes its defaults. The host merges an override
/// in `config.yaml` into its section key by key, so a section here already
/// holds both.
///
/// A bad `provider`, `tags`, or `lead` section is an error for every route,
/// not its defaults. A config mistake must be loud, and each default can do
/// harm: the default provider would send page content to an API that the
/// operator did not choose, the default tag lists would drop the blocked
/// tags, and the default `lead.enabled` is `true`, so a bad section that
/// tried to turn summaries off would turn them on. A bad `workflow` section
/// takes its defaults: no route reads it.
///
/// # Errors
///
/// An error string that names the `provider`, `tags`, or `lead` section
/// when it is not a valid table (for example `provider: host` in
/// `config.yaml`, a string where a table belongs) or has a key of the wrong
/// type.
// Bug b466
pub fn from_sections(get: impl Fn(&str) -> Option<String>) -> Result<PluginConfig, String> {
    Ok(PluginConfig {
        provider: section(&get, "provider")?,
        tags: section(&get, "tags")?,
        lead: section(&get, "lead")?,
        workflow: section(&get, "workflow").unwrap_or_default(),
    })
}

/// One section, parsed from its inline-table value, or its defaults when the
/// section is absent.
fn section<T: DeserializeOwned + Default>(
    get: &impl Fn(&str) -> Option<String>,
    name: &str,
) -> Result<T, String> {
    /// The value under a fixed key, so that the section parses as a
    /// one-line TOML document.
    #[derive(Deserialize)]
    struct One<T> {
        v: T,
    }
    let Some(value) = get(name) else {
        return Ok(T::default());
    };
    toml::from_str::<One<T>>(&format!("v = {value}"))
        .map(|one| one.v)
        .map_err(|e| format!("[config.{name}] is not a valid table: {e}"))
}

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
    /// Provider type: "host", "anthropic", "openai", or "ollama". With
    /// "host", the host owns the model, so `api_key_env`, `model`, and
    /// `base_url` are not read.
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

    // Bug b466: the host serves each section as an inline TOML table.
    #[test]
    fn sections_from_the_host_are_read() {
        let get = |key: &str| match key {
            "provider" => Some(r#"{ model = "m", type = "host" }"#.to_string()),
            "tags" => Some(r#"{ max_tags = 5, preferred_tags = ["rust", "web"] }"#.to_string()),
            _ => None,
        };
        let config = from_sections(get).unwrap();
        assert_eq!(config.provider.provider_type, "host");
        assert_eq!(config.provider.model, "m");
        assert_eq!(config.tags.max_tags, 5);
        assert_eq!(config.tags.preferred_tags, vec!["rust", "web"]);
        assert!(config.lead.enabled, "a missing section keeps its defaults");
    }

    #[test]
    fn no_sections_give_the_defaults() {
        let config = from_sections(|_| None).unwrap();
        assert_eq!(config.provider.provider_type, "anthropic");
    }

    #[test]
    fn a_bad_section_is_an_error_that_names_it() {
        // `provider: host` in config.yaml arrives as the bare string `host`.
        // Falling back to the default provider would send page content to
        // an API that the operator did not choose.
        let get = |key: &str| match key {
            "provider" => Some("host".to_string()),
            "tags" => Some("{ max_tags = 5 }".to_string()),
            _ => None,
        };
        let err = from_sections(get).unwrap_err();
        assert!(err.contains("[config.provider]"), "{err}");
    }

    #[test]
    fn a_bad_lead_section_is_an_error() {
        // A wrong-typed `enabled = "false"` must not turn summaries on.
        let get = |key: &str| (key == "lead").then(|| r#"{ enabled = "false" }"#.to_string());
        let err = from_sections(get).unwrap_err();
        assert!(err.contains("[config.lead]"), "{err}");
    }

    #[test]
    fn a_bad_workflow_section_takes_its_defaults() {
        let get = |key: &str| (key == "workflow").then(|| "auto".to_string());
        assert!(from_sections(get).is_ok());
    }
}
