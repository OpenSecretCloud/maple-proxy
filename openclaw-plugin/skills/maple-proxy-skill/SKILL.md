---
name: maple-proxy-skill
description: Use Maple TEE-backed AI models via the local maple-proxy
metadata: {"openclaw": {"requires": {"config": ["plugins.entries.maple-openclaw-plugin.enabled"]}, "primaryEnv": "MAPLE_API_KEY", "emoji": "🍁"}}
---

# Maple Proxy

The maple-openclaw-plugin manages a local OpenAI-compatible proxy server that forwards requests to Maple's TEE (Trusted Execution Environment) backend. All AI inference runs inside secure enclaves.

## Provider Setup (Recommended)

maple-proxy runs on port **8000** by default -- the same as vLLM. OpenClaw can auto-discover it as a vLLM-compatible provider. To enable:

1. Set `VLLM_API_KEY` to any value (e.g., `"maple-local"`)
2. Do **not** define an explicit `models.providers.vllm` entry
3. OpenClaw will discover models at `http://127.0.0.1:8000/v1/models`
4. Use models as `vllm/<model-id>` (e.g., `vllm/llama3-3-70b`)

Or configure explicitly under `models.providers`:

```json
{
  "models": {
    "providers": {
      "maple": {
        "baseUrl": "http://127.0.0.1:8000/v1",
        "apiKey": "maple-local",
        "api": "openai-completions",
        "models": [
          { "id": "llama3-3-70b", "name": "Llama 3.3 70B" }
        ]
      }
    }
  }
}
```

## Status Tool

Use the `maple_proxy_status` tool to check if the proxy is running, which port it is on, and its health status.

## Embeddings & Memory Search

maple-proxy serves an OpenAI-compatible embeddings endpoint using the `nomic-embed-text` model. You can use this for OpenClaw's memory search so that embeddings are generated inside the TEE -- no cloud embedding provider needed.

To configure memory search with maple-proxy embeddings, add this to your `openclaw.json`:

```json
{
  "agents": {
    "defaults": {
      "memorySearch": {
        "provider": "openai",
        "model": "nomic-embed-text",
        "remote": {
          "baseUrl": "http://127.0.0.1:8000/v1/",
          "apiKey": "maple-local"
        }
      }
    }
  }
}
```

Notes:
- The `apiKey` value can be anything (e.g., `"maple-local"`) since maple-proxy uses the plugin-configured API key for backend auth
- If you changed the plugin port, update the `baseUrl` accordingly
- This replaces the need for a separate OpenAI, Gemini, or Voyage API key for embeddings
- Compatible with OpenClaw's hybrid search (BM25 + vector), session memory indexing, and embedding cache

## Direct API Access

- `GET http://127.0.0.1:8000/v1/models` - List available models
- `POST http://127.0.0.1:8000/v1/chat/completions` - Chat completions (streaming and non-streaming)
- `POST http://127.0.0.1:8000/v1/embeddings` - Generate embeddings (model: `nomic-embed-text`)
- `GET http://127.0.0.1:8000/health` - Health check

## Port Override

The default port is 8000. If something else uses port 8000, override it in plugin config:

```json
{
  "plugins": {
    "entries": {
      "maple-openclaw-plugin": {
        "config": { "port": 8200 }
      }
    }
  }
}
```

If you change the port, update your `models.providers` base URL to match.

## Authentication

Authentication is handled automatically by the plugin via the configured API key. No per-request auth headers are needed from the agent.
