---
name: maple-proxy-skill
description: Use Maple TEE-backed AI models via the local maple-proxy
metadata: {"openclaw": {"requires": {"config": ["plugins.entries.maple-openclaw-plugin.enabled"]}, "primaryEnv": "MAPLE_API_KEY", "emoji": "🍁"}}
---

# Maple Proxy

The maple-openclaw-plugin manages a local OpenAI-compatible proxy server that forwards requests to Maple's TEE (Trusted Execution Environment) backend. All AI inference runs inside secure enclaves.

## Setup

### 1. Add the Maple provider

Add a `maple` provider to your `openclaw.json` with your Maple API key and the models you want to use. maple-proxy runs on port **8787** by default.

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

Use the same Maple API key you configured in the plugin config -- maple-proxy forwards the `Authorization: Bearer` header to the TEE backend for authentication.

To discover available models, use the `maple_proxy_status` tool or call `GET http://127.0.0.1:8787/v1/models` directly.

### 2. Add models to the allowlist

If you have an `agents.defaults.models` section in your config, you must add the maple models you want to use. If you don't have this section at all, skip this step -- all models are allowed by default.

Add each model you want to use as `maple/<model-id>`. Check available models via `GET http://127.0.0.1:8787/v1/models` or the `maple_proxy_status` tool.

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

### 3. Restart the gateway

Restart the OpenClaw gateway to pick up the new provider and model config.

## Using Maple Models

Use maple models by prefixing with `maple/`:

- `maple/kimi-k2-5` (recommended)
- `maple/deepseek-r1-0528`
- `maple/gpt-oss-120b`
- `maple/llama-3.3-70b`
- `maple/qwen3-vl-30b`

To spawn a subagent on a Maple model:

```
Use sessions_spawn with model: "maple/kimi-k2-5" to run tasks on Maple TEE models.
```

## Status Tool

Use the `maple_proxy_status` tool to check if the proxy is running, which port it is on, its health status, and the available models endpoint.

## Embeddings & Memory Search

maple-proxy serves an OpenAI-compatible embeddings endpoint using the `nomic-embed-text` model. You can use this for OpenClaw's memory search so that embeddings are generated inside the TEE — no cloud embedding provider needed.

### 1. Enable the memory-core plugin

The `memory_search` and `memory_get` tools are provided by OpenClaw's `memory-core` plugin. It ships as a stock plugin but must be explicitly enabled. Add it to `plugins.allow` and `plugins.entries`:

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

This requires a **full gateway restart** (not just SIGUSR1) since it's a plugin change.

### 2. Configure memorySearch to use maple-proxy

Point `memorySearch.remote` at the local maple-proxy endpoint. **Important**: the `model` field must be `nomic-embed-text` (without a `maple/` provider prefix) — the proxy does not strip provider prefixes for embedding requests.

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

Use the same Maple API key you configured in the plugin config. This replaces the need for a separate OpenAI, Gemini, or Voyage API key for embeddings.

> **Common mistake**: Setting the model to `maple/nomic-embed-text` will cause 400 errors from the proxy. Use `nomic-embed-text` (no prefix).

### 3. Restart and reindex

After updating the config, do a full gateway restart, then build the vector index:

```bash
# Full restart (plugin changes require this)
systemctl restart openclaw.service

# Index memory files and generate embeddings
openclaw memory index --verbose

# Verify everything is working
openclaw memory status --deep
```

The status output should show:
- **Provider**: `openai` (this is the API format, not the actual provider)
- **Model**: `nomic-embed-text`
- **Embeddings**: `available` (not `unavailable`)
- **Vector**: `ready`

### 4. Test with the CLI and tool

Test from the command line first:

```bash
openclaw memory search "your query here"
```

Once that works, the `memory_search` tool will also be available to the agent in chat. The agent can call `memory_search` to semantically search across `MEMORY.md` and `memory/*.md` files, with results ranked by relevance and cited with source paths.

### Troubleshooting

- **"memory slot plugin not found"** in logs → `memory-core` is not in `plugins.allow` or `plugins.entries`, or hasn't been restarted after adding it
- **Embeddings 400 error** → model name includes provider prefix (`maple/nomic-embed-text`), change to `nomic-embed-text`
- **Embeddings 401 error** → wrong API key, or key is a literal string like `${MAPLE_API_KEY}` instead of the actual key value
- **"Batch: disabled"** in status → embeddings failed too many times, fix the config and restart to reset the failure counter
- **Only 1/7 files indexed** → embeddings were failing, fix config, restart, then run `openclaw memory index --verbose`

## Direct API Access

- `GET http://127.0.0.1:8787/v1/models` - List available models
- `POST http://127.0.0.1:8787/v1/chat/completions` - Chat completions (streaming and non-streaming)
- `POST http://127.0.0.1:8787/v1/embeddings` - Generate embeddings (model: `nomic-embed-text`)
- `GET http://127.0.0.1:8787/health` - Health check

## Port Override

The default port is 8787. To change it:

```json
{
  "plugins": {
    "entries": {
      "maple-openclaw-plugin": {
        "config": { "port": 9000 }
      }
    }
  }
}
```

If you change the port, update your `models.providers.maple.baseUrl` and `memorySearch.remote.baseUrl` to match.

## Configuration Changes

Plugin config changes (port, API key, backend URL) require a full gateway restart to take effect. Model and provider config changes hot-apply without a restart.
