---
name: maple-proxy-skill
description: Use Maple TEE-backed AI models via the local maple-proxy
metadata: {"openclaw": {"requires": {"config": ["plugins.entries.maple-proxy-openclaw-plugin.enabled"]}, "primaryEnv": "MAPLE_API_KEY", "emoji": "🍁"}}
---

# Maple Proxy

The maple-proxy plugin manages a local OpenAI-compatible proxy server that forwards requests to Maple's TEE (Trusted Execution Environment) backend. All AI inference runs inside secure enclaves.

## Available Endpoints

- `GET http://127.0.0.1:8080/v1/models` - List available models
- `POST http://127.0.0.1:8080/v1/chat/completions` - Chat completions (streaming and non-streaming)
- `GET http://127.0.0.1:8080/health` - Health check

## Usage

The proxy is OpenAI-compatible. Use any OpenAI client library pointed at `http://127.0.0.1:8080`. The port may differ if 8080 was busy -- check the plugin logs for the actual port.

Authentication is handled automatically by the plugin via the configured API key.
