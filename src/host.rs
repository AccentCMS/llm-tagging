//! Thin ergonomic wrappers over the host capabilities the generated component
//! bindings expose, so the rest of the plugin does not reference `bindings::`
//! directly. These replace the `extism_pdk::config::get` / `extism_pdk::http`
//! calls the Extism version made.

use crate::bindings::accent::plugin::config;
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
