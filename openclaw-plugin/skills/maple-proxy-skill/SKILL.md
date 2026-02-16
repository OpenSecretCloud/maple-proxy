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
          { "id": "llama-3.3-70b", "name": "Llama 3.3 70B" }
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
        "maple/llama-3.3-70b": {}
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
- `maple/llama-3.3-70b`

To spawn a subagent on a Maple model:

```
Use sessions_spawn with model: "maple/kimi-k2-5" to run tasks on Maple TEE models.
```

## Status Tool

Use the `maple_proxy_status` tool to check if the proxy is running, which port it is on, its health status, and the available models endpoint.

## Embeddings & Memory Search

maple-proxy serves an OpenAI-compatible embeddings endpoint using the `nomic-embed-text` model. You can use this for OpenClaw's memory search so that embeddings are generated inside the TEE -- no cloud embedding provider needed.

```json
{
  "agents": {
    "defaults": {
      "memorySearch": {
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

Use the same Maple API key here. This replaces the need for a separate OpenAI, Gemini, or Voyage API key for embeddings. Compatible with OpenClaw's hybrid search (BM25 + vector), session memory indexing, and embedding cache.

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
