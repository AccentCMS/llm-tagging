# LLM Tagging Plugin

AI-powered document tagging and lead generation for Accent CMS.

This WASM plugin asks a language model to generate tags and lead summaries
for your CMS content. The model is either the one you configure for Accent
(`type = "host"`, for example Ollama on the same machine), or a public API
(Anthropic, OpenAI, or Ollama on a public address).

## Requirements

- Accent CMS built with `--features plugins`
- Rust toolchain with `wasm32-wasip1` target (for building from source)
- A model: `plugins.model` in config.yaml for the `host` provider (Accent
  0.29 or newer), or an LLM API key (Anthropic, OpenAI), or an Ollama server
  on a public address

## Quick Start

### 1. Set your API key

```bash
export LLM_API_KEY="sk-ant-..."  # Anthropic key
# or
export LLM_API_KEY="sk-..."      # OpenAI key
```

### 2. Configure the provider

Edit `plugins/llm-tagging/plugin.toml`:

```toml
[config.provider]
type = "anthropic"          # "host", "anthropic", "openai", or "ollama"
api_key_env = "LLM_API_KEY" # env var name holding the key
model = "claude-3-haiku-20240307"
```

You can also set keys in `config.yaml`. Accent merges them into the
section of `plugin.toml` key by key, so the other keys stay:

```yaml
plugins:
  config:
    llm-tagging:
      provider:
        type: host
```

### 3. Build the plugin

Built with [`cargo-component`][cargo-component] into a Component-Model
component implementing the `model-api-plugin` world:

```bash
cd plugins/llm-tagging
cargo install cargo-component   # one-time setup
cargo component build --release

# Copy the compiled component into the plugin directory as plugin.wasm
cp target/wasm32-wasip1/release/llm_tagging.wasm plugin.wasm
```

[cargo-component]: https://github.com/bytecodealliance/cargo-component

### 4. Start Accent CMS

```bash
cargo run --features plugins -- serve
```

## API Endpoints

### Analyze a single page

```bash
curl -X POST http://localhost:4400/api/llm/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "page_path": "/blog/my-post",
    "title": "Building a CMS in Rust",
    "content": "# Building a CMS in Rust\n\nThis guide covers...",
    "types": ["tags", "lead"]
  }'
```

Response:

```json
{
  "tags": ["rust", "cms", "web-development"],
  "lead": "Learn how to build a high-performance CMS using Rust.",
  "confidence": 0.9,
  "model": "claude-3-haiku-20240307",
  "page_path": "/blog/my-post"
}
```

### Batch analyze multiple pages

```bash
curl -X POST http://localhost:4400/api/llm/batch \
  -H "Content-Type: application/json" \
  -d '{
    "pages": [
      {"page_path": "/blog/post-1", "title": "Post 1", "content": "..."},
      {"page_path": "/blog/post-2", "title": "Post 2", "content": "..."}
    ],
    "types": ["tags"]
  }'
```

### Get taxonomy suggestions

```bash
curl http://localhost:4400/api/llm/taxonomy
```

Returns the preferred tags configured in `plugin.toml`.

## Template Filter

Use the `format_tags` filter in your Jinja templates:

```jinja
{{ page.tags | format_tags }}
{# Output: "rust, cms, web-development" #}
```

## Configuration Reference

All settings live in `plugin.toml` under `[config]`. Keys under
`plugins.config.llm-tagging` in `config.yaml` override the same keys.

### Provider

| Key | Default | Description |
|-----|---------|-------------|
| `provider.type` | `"anthropic"` | LLM backend: `"host"`, `"anthropic"`, `"openai"`, `"ollama"`. With `"host"`, the other provider keys are not read |
| `provider.api_key_env` | `"LLM_API_KEY"` | Environment variable holding the API key |
| `provider.model` | `"claude-3-haiku-20240307"` | Model identifier |
| `provider.base_url` | `""` | Custom endpoint (for self-hosted APIs); required for `"ollama"` |
| `provider.timeout_seconds` | `30` | Request timeout |

### Tags

| Key | Default | Description |
|-----|---------|-------------|
| `tags.enabled` | `true` | Enable tag generation |
| `tags.max_tags` | `8` | Maximum tags per page |
| `tags.min_tags` | `3` | Minimum tags per page |
| `tags.style` | `"lowercase-hyphen"` | Format: `"lowercase-hyphen"`, `"lowercase"`, `"title-case"` |
| `tags.preferred_tags` | `[]` | Tags to prefer for consistency |
| `tags.blocked_tags` | `[]` | Tags to never generate |

### Lead

| Key | Default | Description |
|-----|---------|-------------|
| `lead.enabled` | `true` | Enable lead/summary generation |
| `lead.max_length` | `200` | Maximum character length |
| `lead.min_length` | `50` | Minimum character length |
| `lead.style` | `"informative"` | Style: `"informative"`, `"engaging"`, `"technical"`, `"casual"` |

### Workflow

| Key | Default | Description |
|-----|---------|-------------|
| `workflow.auto_analyze` | `false` | Mark pages for analysis on load |
| `workflow.skip_if_exists` | `true` | Skip if tags/lead already exist |

## Supported Providers

### Host (a model on this machine or your network)

Set `type = "host"`. Accent calls the model that you configure in
`config.yaml` for the plugin:

```yaml
plugins:
  model:
    provider: openai                  # Ollama serves the OpenAI protocol
    endpoint: http://localhost:11434
    model: llama3.1
```

The plugin sends only the prompt. Accent owns the address, the model name,
and the key (`ACCENTCMS_PLUGIN_MODEL_API_KEY`, if the server needs one). This
is why a model on `localhost` or the LAN works here: the plugin never names
an address. The manifest declares `host_services = ["model"]` for this. The
`host` provider needs Accent 0.29 or newer.

### Anthropic (default)

Uses the Messages API. Set `type = "anthropic"` and provide a key via the
`LLM_API_KEY` environment variable.

### OpenAI

Uses the Chat Completions API. Set `type = "openai"` and provide a key.
Compatible with any OpenAI-compatible endpoint (set `base_url`).

### Ollama on a public address

No API key needed. Set `type = "ollama"` and point `base_url` at an Ollama
server on a public address, then add that host to `allowed_hosts` in
`plugin.toml`. `base_url` has no default.

This provider calls Ollama over the plugin's own outbound HTTP, which refuses
every loopback and private address, including `localhost:11434`. For Ollama
on the same machine or the LAN, use the `host` provider.

```toml
[config.provider]
type = "ollama"
model = "llama3"
base_url = "https://ollama.example.com"
```

## Architecture

The plugin is a WebAssembly Component-Model component (`model-api-plugin`
world). The `host` provider calls the operator's model through the
`host-services` capability. The other providers call LLM APIs through the
`outbound-http` capability, and `allowed_hosts` in `plugin.toml` lists the
domains they can reach. Both capabilities are granted structurally, because
the world imports them.

```
wit/                # Vendored slice of the Accent plugin contract + world.wit
plugin.toml         # Declares routes, filters, allowed_hosts, host_services
src/lib.rs          # Guest impls: routes::handle + filters::apply
src/host.rs         # Wrappers over the config, outbound-http, and host-services capabilities
src/provider.rs     # LLM provider abstraction (host/Anthropic/OpenAI/Ollama)
src/prompts.rs      # Prompt template builders
src/config.rs       # Configuration parsing
src/types.rs        # Request/response JSON types
```

Unlike the Extism version, the component does not export a content hook at all:
LLM API calls take 2-30 seconds and the synchronous render path would block, so
analysis is triggered only via the route handlers. (A page-load hook returns
when page-load hooks can run asynchronously -- see the E034 post-parity work.)

## Phase 2 (Planned)

- Workflow engine with async job queue
- Result caching to avoid redundant API calls
- Frontmatter write-back (auto-update page tags)
- Host runtime context awareness (production vs dev mode)
