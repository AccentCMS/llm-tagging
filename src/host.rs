//! Thin ergonomic wrappers over the host capabilities the generated component
//! bindings expose, so the rest of the plugin does not reference `bindings::`
//! directly. These replace the `extism_pdk::config::get` / `extism_pdk::http`
//! calls the Extism version made.

use crate::bindings::accent::plugin::config;
use crate::bindings::accent::plugin::host_services::{
    self, HostError, ModelMessage, ModelRequest, ModelRole,
};
use crate::bindings::accent::plugin::outbound_http::{self, HttpRequest};

/// Read a single merged-config value (capability `config`, always granted).
pub fn config_get(key: &str) -> Option<String> {
    config::get(key)
}

/// POST a JSON body to `url` through the `outbound-http` capability and return
/// `(status, body_text)`. The host enforces the manifest `allowed-hosts`
/// allowlist; a request to an undeclared host fails with `host-not-allowed`.
pub fn http_post_json(
    url: &str,
    headers: &[(&str, &str)],
    body: String,
) -> Result<(u16, String), String> {
    let req = HttpRequest {
        method: "POST".to_string(),
        url: url.to_string(),
        headers: headers
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
        body: Some(body.into_bytes()),
    };
    let res = outbound_http::fetch(&req).map_err(|e| format!("{e:?}"))?;
    let text = String::from_utf8(res.body)
        .map_err(|e| format!("response body is not valid UTF-8: {e}"))?;
    Ok((res.status, text))
}

/// Ask the operator's model through the `host-services` capability and
/// return the answer text.
///
/// The host owns the endpoint, the model, and the key (`plugins.model` in
/// config.yaml), so this call carries only the prompt. The manifest must
/// declare `host_services = ["model"]`.
pub fn host_model_complete(prompt: &str, max_tokens: u32) -> Result<String, String> {
    let req = ModelRequest {
        messages: vec![ModelMessage {
            role: ModelRole::User,
            content: prompt.to_string(),
        }],
        max_tokens: Some(max_tokens),
    };
    host_services::model_complete(&req)
        .map(|answer| answer.text)
        .map_err(|e| match e {
            // The payload is exactly the scope when the manifest lacks it,
            // and `scope: reason` when the call site may not use it.
            HostError::NotPermitted(detail) if detail == "model" => {
                "the host model needs `host_services = [\"model\"]` in plugin.toml".to_string()
            }
            HostError::NotPermitted(detail) => format!("the host refused the model call: {detail}"),
            HostError::NotConfigured(detail) => {
                format!("the host has no model for plugins: {detail}")
            }
            HostError::InvalidRequest(detail) => format!("the host refused the request: {detail}"),
            HostError::RateLimited(detail) => format!("the host model is rate-limited: {detail}"),
            HostError::Internal(detail) => format!("the host model call failed: {detail}"),
        })
}
