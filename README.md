# llm-tagging

An [Accent CMS](https://accentcms.dev) plugin that generates tags and a summary
lead for a page by asking an LLM provider of your choosing.

## Install

```
accent plugin install llm-tagging
```

## What it does

| | |
|---|---|
| Routes | `POST /api/llm/analyze`, `POST /api/llm/batch`, `GET /api/llm/taxonomy` |
| Filter | `format_tags` |
| Models | contributes `llm-article` |

Analysis runs through the routes rather than a render hook: an LLM call takes
seconds, and the synchronous render path would block on it.

## Configuration

Providers are configured in `plugin.toml` under `[config.provider]` —
Anthropic, OpenAI, or any compatible endpoint via `base_url`.

**The API key is never stored in configuration.** `api_key_env` names an
environment variable, and the plugin reads the key from there.

Tag style, count bounds, preferred and blocked tags, and the lead's length and
tone are all configurable; see the comments in `plugin.toml`.

## Building

This is a WebAssembly Component-Model component. Plain `cargo build` will not
work: the bindings under `src/bindings.rs` are generated from `wit/` by
`cargo-component`.

```
rustup target add wasm32-wasip1
cargo install cargo-component --locked --version 0.21.1
cargo component build --release
```

## Licence

MIT. See [LICENSE](LICENSE).
