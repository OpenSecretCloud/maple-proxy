# @opensecret/maple-openclaw-plugin

OpenClaw plugin that automatically downloads, configures, and runs [maple-proxy](https://github.com/OpenSecretCloud/maple-proxy) as a background service. All AI inference runs inside Maple's TEE (Trusted Execution Environment) secure enclaves.

## Quick Start (Recommended)

Install the plugin and let your agent handle the rest:

```bash
openclaw plugins install @opensecret/maple-openclaw-plugin
```

Then tell your agent:

> Install and configure maple-proxy with my API key: `YOUR_MAPLE_API_KEY`

The plugin bundles a skill that teaches the agent how to set up the maple provider, configure models, and enable embeddings. After a gateway restart, the agent will have all the context it needs from the skill to complete the setup. If the plugin isn't configured yet, the `maple_proxy_status` tool also returns step-by-step instructions.

## Manual Setup

If you prefer to configure everything yourself, follow these steps after installing the plugin.

### 1. Configure the plugin

Set your Maple API key in `openclaw.json`:

```json
{
  "plugins": {
    "entries": {
      "maple-openclaw-plugin": {
        "enabled": true,
        "config": {
          "apiKey": "YOUR_MAPLE_API_KEY"
        }
      }
    }
  }
}
```

### 2. Add the Maple provider

Add a `maple` provider so OpenClaw can route requests to the local proxy (default port **8787**):

```json
{
  "models": {
    "providers": {
      "maple": {
        "baseUrl": "http://127.0.0.1:8787/v1",
        "apiKey": "YOUR_MAPLE_API_KEY",
        "api": "openai-completions",
        "models": [
          { "id": "kimi-k2-5", "name": "Kimi K2.5 (recommended)" },
          { "id": "deepseek-r1-0528", "name": "DeepSeek R1" },
          { "id": "gpt-oss-120b", "name": "GPT-OSS 120B" },
          { "id": "llama-3.3-70b", "name": "Llama 3.3 70B" },
          { "id": "qwen3-vl-30b", "name": "Qwen3 VL 30B" }
        ]
      }
    }
  }
}
```

Use the same Maple API key in both places. To discover all available models, call `GET http://127.0.0.1:8787/v1/models` after startup.

### 3. Add models to the allowlist (if applicable)

If you have an `agents.defaults.models` section in your config, add the maple models you want. If you don't have this section, skip this step -- all models are allowed by default.

```json
{
  "agents": {
    "defaults": {
      "models": {
        "maple/kimi-k2-5": {},
        "maple/deepseek-r1-0528": {},
        "maple/gpt-oss-120b": {},
        "maple/llama-3.3-70b": {},
        "maple/qwen3-vl-30b": {}
      }
    }
  }
}
```

### 4. Restart the gateway

```bash
systemctl restart openclaw.service
```

Plugin config changes always require a full gateway restart. Model and provider config changes hot-apply without a restart.

## Usage

Use maple models by prefixing with `maple/`:

- `maple/kimi-k2-5` (recommended)
- `maple/deepseek-r1-0528`
- `maple/gpt-oss-120b`
- `maple/llama-3.3-70b`
- `maple/qwen3-vl-30b`

The plugin also registers a `maple_proxy_status` tool that shows the proxy's health, port, version, and available endpoints. If the plugin isn't configured yet, the tool returns setup instructions.

## Embeddings & Memory Search

maple-proxy serves an OpenAI-compatible embeddings endpoint using the `nomic-embed-text` model. You can use this for OpenClaw's memory search so embeddings are generated inside the TEE -- no cloud embedding provider needed.

### Enable the memory-core plugin

The `memory_search` and `memory_get` tools come from OpenClaw's `memory-core` plugin. It ships as a stock plugin but **must be explicitly enabled**:

```json
{
  "plugins": {
    "allow": ["memory-core"],
    "entries": {
      "memory-core": {
        "enabled": true
      }
    }
  }
}
```

### Configure memorySearch

> **Important**: The model field must be `nomic-embed-text` (without a `maple/` prefix). Using `maple/nomic-embed-text` will cause 400 errors.

```json
{
  "agents": {
    "defaults": {
      "memorySearch": {
        "enabled": true,
        "provider": "openai",
        "model": "nomic-embed-text",
        "remote": {
          "baseUrl": "http://127.0.0.1:8787/v1/",
          "apiKey": "YOUR_MAPLE_API_KEY"
        }
      }
    }
  }
}
```

### Restart and reindex

```bash
systemctl restart openclaw.service
openclaw memory index --verbose
openclaw memory status --deep
```

The status output should show **Embeddings: available** and **Vector: ready**.

### Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| "memory slot plugin not found" | `memory-core` not enabled | Add to `plugins.allow` and `plugins.entries`, restart |
| Embeddings 400 error | Model has provider prefix | Change `maple/nomic-embed-text` to `nomic-embed-text` |
| Embeddings 401 error | Wrong API key | Check the key is the actual value, not a placeholder |
| "Batch: disabled" in status | Too many embedding failures | Fix config, restart to reset failure counter |
| Only some files indexed | Embeddings were failing during indexing | Fix config, restart, run `openclaw memory index --verbose` |

## Plugin Config Options

| Option | Default | Description |
|---|---|---|
| `apiKey` | (required) | Your Maple API key |
| `port` | `8787` | Local port for the proxy |
| `backendUrl` | `https://enclave.trymaple.ai` | Maple TEE backend URL |
| `debug` | `false` | Enable debug logging |
| `version` | (latest) | Pin to a specific maple-proxy version |

## Updating

```bash
openclaw plugins update maple-openclaw-plugin
```

## Direct API Access

- `GET http://127.0.0.1:8787/v1/models` -- List available models
- `POST http://127.0.0.1:8787/v1/chat/completions` -- Chat completions (streaming and non-streaming)
- `POST http://127.0.0.1:8787/v1/embeddings` -- Generate embeddings (model: `nomic-embed-text`)
- `GET http://127.0.0.1:8787/health` -- Health check

## License

MIT
