# 🍁 Maple Proxy

A lightweight OpenAI-compatible proxy server for Maple/OpenSecret's TEE infrastructure. Works with **any** OpenAI client library while providing the security and privacy benefits of Trusted Execution Environment (TEE) processing.

## 🚀 Features

- **100% OpenAI Compatible** - Drop-in replacement for OpenAI API
- **Secure TEE Processing** - All requests processed in secure enclaves
- **Streaming Support** - Full Server-Sent Events streaming for chat completions
- **Flexible Authentication** - Environment variables or per-request API keys
- **Zero Client Changes** - Works with existing OpenAI client code
- **Lightweight** - Minimal overhead, maximum performance
- **CORS Support** - Ready for web applications

## 📦 Installation

```bash
git clone <repository>
cd maple-proxy
cargo build --release
```

## ⚙️ Configuration

Set environment variables or use command-line arguments:

```bash
# Environment Variables
export MAPLE_HOST=127.0.0.1                    # Server host (default: 127.0.0.1)
export MAPLE_PORT=3000                         # Server port (default: 3000)
export MAPLE_BACKEND_URL=http://localhost:3000         # Maple backend URL (prod: https://enclave.trymaple.ai)
export MAPLE_API_KEY=your-maple-api-key        # Default API key (optional)
export MAPLE_DEBUG=true                        # Enable debug logging
export MAPLE_ENABLE_CORS=true                  # Enable CORS
```

Or use CLI arguments:
```bash
cargo run -- --host 0.0.0.0 --port 8080 --backend-url https://enclave.trymaple.ai
```

## 🛠️ Usage

### Start the Server

```bash
cargo run
```

You should see:
```
🚀 Maple Proxy Server started successfully!
📋 Available endpoints:
   GET  /health              - Health check
   GET  /v1/models           - List available models
   POST /v1/chat/completions - Create chat completions (streaming)
```

### API Endpoints

#### List Models
```bash
curl http://localhost:8080/v1/models \
  -H "Authorization: Bearer YOUR_MAPLE_API_KEY"
```

#### Chat Completions (Streaming)
```bash
curl -N http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_MAPLE_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3-3-70b",
    "messages": [
      {"role": "user", "content": "Write a haiku about technology"}
    ],
    "stream": true
  }'
```

**Note:** Maple currently only supports streaming responses.

## 💻 Client Examples

### Python (OpenAI Library)

```python
import openai

client = openai.OpenAI(
    api_key="YOUR_MAPLE_API_KEY",
    base_url="http://localhost:8080/v1"
)

# Streaming chat completion
stream = client.chat.completions.create(
    model="llama3-3-70b",
    messages=[{"role": "user", "content": "Hello, world!"}],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content is not None:
        print(chunk.choices[0].delta.content, end="")
```

### JavaScript/Node.js

```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: 'YOUR_MAPLE_API_KEY',
  baseURL: 'http://localhost:8080/v1',
});

const stream = await openai.chat.completions.create({
  model: 'llama3-3-70b',
  messages: [{ role: 'user', content: 'Hello!' }],
  stream: true,
});

for await (const chunk of stream) {
  process.stdout.write(chunk.choices[0]?.delta?.content || '');
}
```

### cURL

```bash
# Health check
curl http://localhost:8080/health

# List models
curl http://localhost:8080/v1/models \
  -H "Authorization: Bearer YOUR_MAPLE_API_KEY"

# Streaming chat completion
curl -N http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer YOUR_MAPLE_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3-3-70b",
    "messages": [{"role": "user", "content": "Tell me a joke"}],
    "stream": true
  }'
```

## 🔐 Authentication

Maple Proxy supports two authentication methods:

### 1. Environment Variable (Default)
Set `MAPLE_API_KEY` - all requests will use this key by default:
```bash
export MAPLE_API_KEY=your-maple-api-key
cargo run
```

### 2. Per-Request Authorization Header
Override the default key or provide one if not set:
```bash
curl -H "Authorization: Bearer different-api-key" ...
```

## 🌐 CORS Support

Enable CORS for web applications:
```bash
export MAPLE_ENABLE_CORS=true
cargo run
```

## 🐳 Docker Deployment

### Quick Start with Docker

```bash
# Build the image
just docker-build

# Run the container
just docker-run
```

### Production Docker Setup

1. **Build the optimized image:**
```bash
docker build -t maple-proxy:latest .
```

2. **Run with docker-compose:**
```bash
# Copy the example environment file
cp .env.example .env

# Edit .env with your configuration
vim .env

# Start the service
docker-compose up -d
```

### 🔒 Security Note for Public Deployments

When deploying Maple Proxy on a public network:

- **DO NOT** set `MAPLE_API_KEY` in the container environment
- Instead, require clients to pass their API key with each request:

```python
# Client-side authentication for public proxy
client = OpenAI(
    base_url="https://your-proxy.example.com/v1",
    api_key="user-specific-maple-api-key"  # Each user provides their own key
)
```

This ensures:
- Users' API keys remain private
- Multiple users can share the same proxy instance
- No API keys are exposed in container configurations

### Docker Commands

```bash
# Build image
just docker-build

# Run interactively
just docker-run

# Run in background
just docker-run-detached

# View logs
just docker-logs

# Stop container
just docker-stop

# Use docker-compose
just compose-up
just compose-logs
just compose-down
```

### Container Configuration

The Docker image:
- Uses multi-stage builds for minimal size (~130MB)
- Runs as non-root user for security
- Includes health checks
- Optimizes dependency caching with cargo-chef
- Supports both x86_64 and ARM architectures

### Environment Variables for Docker

```yaml
# docker-compose.yml environment section
environment:
  - MAPLE_BACKEND_URL=https://enclave.trymaple.ai  # Production backend
  - MAPLE_ENABLE_CORS=true                         # Enable for web apps
  - RUST_LOG=info                                  # Logging level
  # - MAPLE_API_KEY=xxx                            # Only for private deployments!
```

## 🔧 Development

### Build
```bash
cargo build
```

### Run with Debug Logging
```bash
export MAPLE_DEBUG=true
cargo run
```

### Run Tests
```bash
cargo test
```

## 📊 Supported Models

Maple Proxy supports all models available in the Maple/OpenSecret platform, including:
- `llama3-3-70b` - Llama 3.3 70B parameter model
- And many others - check `/v1/models` endpoint for current list

## 🔍 Troubleshooting

### Common Issues

**"No API key provided"**
- Set `MAPLE_API_KEY` environment variable or provide `Authorization: Bearer <key>` header

**"Failed to establish secure connection"**
- Check your `MAPLE_BACKEND_URL` is correct
- Ensure your API key is valid
- Check network connectivity

**Connection refused**
- Make sure the server is running on the specified host/port
- Check firewall settings

### Debug Mode

Enable debug logging for detailed information:
```bash
export MAPLE_DEBUG=true
cargo run
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   OpenAI Client │───▶│   Maple Proxy   │───▶│  Maple Backend  │
│   (Python/JS)   │    │   (localhost)   │    │      (TEE)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

1. **Client** makes standard OpenAI API calls to localhost
2. **Maple Proxy** handles authentication and TEE handshake
3. **Requests** are securely forwarded to Maple's TEE infrastructure
4. **Responses** are streamed back to the client in OpenAI format

## 📝 License

MIT License - see LICENSE file for details.

## 🤝 Contributing

Contributions welcome! Please feel free to submit a Pull Request.
