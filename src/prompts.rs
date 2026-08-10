//! Prompt template generation for LLM analysis (Feature f026a).
//!
//! Builds structured prompts for tag and lead generation, instructing
//! the LLM to return JSON-formatted responses that can be reliably
//! parsed by the plugin.

use crate::config::{LeadConfig, TagsConfig};

/// Build a tag generation prompt for the given page content.
///
/// The prompt asks the LLM to return a JSON object with a `tags` array
/// and optional `reasoning` string. Includes constraints from the tag
/// configuration (min/max count, style, preferred/blocked lists).
pub fn build_tag_prompt(title: &str, path: &str, content: &str, config: &TagsConfig) -> String {
    let preferred = if config.preferred_tags.is_empty() {
        "none specified".to_string()
    } else {
        config.preferred_tags.join(", ")
    };

    let blocked = if config.blocked_tags.is_empty() {
        "none".to_string()
    } else {
        config.blocked_tags.join(", ")
    };

    // Truncate content to ~4000 chars to stay within context limits.
    let truncated = if content.len() > 4000 {
        &content[..4000]
    } else {
        content
    };

    format!(
        r#"You are analyzing a document for a content management system. Generate relevant tags for the following content.

Rules:
- Generate between {min} and {max} tags
- Tags should be in {style} format (e.g., "web-development" not "Web Development")
- Focus on the main topics, technologies, and concepts
- Avoid overly generic tags like "article" or "post"
- Prefer these existing site tags when relevant: {preferred}
- Never use these blocked tags: {blocked}

Document Title: {title}
Document Path: {path}

Content:
---
{content}
---

Respond with ONLY a JSON object (no markdown fencing):
{{"tags": ["tag1", "tag2"], "reasoning": "Brief explanation of tag choices"}}"#,
        min = config.min_tags,
        max = config.max_tags,
        style = config.style,
        preferred = preferred,
        blocked = blocked,
        title = title,
        path = path,
        content = truncated,
    )
}

/// Build a lead/summary generation prompt for the given page content.
///
/// The prompt asks the LLM to return a JSON object with a `lead` string
/// and optional `alternatives` array. Respects length and style constraints.
pub fn build_lead_prompt(title: &str, path: &str, content: &str, config: &LeadConfig) -> String {
    // Truncate content to ~4000 chars to stay within context limits.
    let truncated = if content.len() > 4000 {
        &content[..4000]
    } else {
        content
    };

    format!(
        r#"You are writing a compelling lead/summary for a blog post listing page. The lead should entice readers to click through and read the full article.

Rules:
- Maximum {max_length} characters
- Minimum {min_length} characters
- Style: {style}
- Capture the essence of the article
- Include a hook or key benefit
- Do not use clickbait or sensationalism

Document Title: {title}
Document Path: {path}

Content:
---
{content}
---

Respond with ONLY a JSON object (no markdown fencing):
{{"lead": "Your generated lead text here", "alternatives": ["Alternative 1", "Alternative 2"]}}"#,
        max_length = config.max_length,
        min_length = config.min_length,
        style = config.style,
        title = title,
        path = path,
        content = truncated,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_prompt_includes_constraints() {
        let config = TagsConfig {
            enabled: true,
            max_tags: 5,
            min_tags: 2,
            style: "lowercase-hyphen".to_string(),
            preferred_tags: vec!["rust".to_string(), "web".to_string()],
            blocked_tags: vec!["article".to_string()],
        };
        let prompt = build_tag_prompt("My Post", "/blog/my-post", "Some content", &config);
        assert!(prompt.contains("between 2 and 5 tags"));
        assert!(prompt.contains("lowercase-hyphen"));
        assert!(prompt.contains("rust, web"));
        assert!(prompt.contains("article"));
        assert!(prompt.contains("My Post"));
        assert!(prompt.contains("/blog/my-post"));
    }

    #[test]
    fn test_tag_prompt_truncates_long_content() {
        let long_content = "a".repeat(5000);
        let config = TagsConfig::default();
        let prompt = build_tag_prompt("Title", "/path", &long_content, &config);
        // The prompt should contain at most ~4000 chars of content.
        assert!(prompt.len() < 5500);
    }

    #[test]
    fn test_lead_prompt_includes_constraints() {
        let config = LeadConfig {
            enabled: true,
            max_length: 150,
            min_length: 30,
            style: "engaging".to_string(),
        };
        let prompt = build_lead_prompt("My Post", "/blog/my-post", "Some content", &config);
        assert!(prompt.contains("Maximum 150 characters"));
        assert!(prompt.contains("Minimum 30 characters"));
        assert!(prompt.contains("engaging"));
    }
}
