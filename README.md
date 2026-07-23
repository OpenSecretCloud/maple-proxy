# 🍁 Maple Proxy

A lightweight proxy for Maple/OpenSecret's OpenAI-compatible inference
endpoints, with the security and privacy benefits of Trusted Execution
Environment (TEE) processing.

## 🚀 Features

- **OpenAI-Compatible Surface** - Models, chat completions, and embeddings endpoints
- **Attested TEE Transport** - The OpenSecret SDK establishes an attested,
  encrypted channel before inference requests are forwarded
- **Lossless Chat Parameters** - Provider-specific request fields pass through unchanged
- **Streaming and Non-Streaming** - Supports both chat completion response modes
- **Flexible Authentication** - Environment variables or per-request API keys
- **Familiar Clients** - Point compatible OpenAI clients at the proxy base URL
- **Lightweight** - Minimal overhead, maximum performance
- **CORS Support** - Ready for web applications

## 📦 Installation

### As a Binary

```bash
git clone <repository>
cd maple-proxy
cargo build --release
```

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
maple-proxy = { git = "https://github.com/opensecretcloud/maple-proxy" }
# Or if published to crates.io:
# maple-proxy = "0.2.0"
```

## ⚙️ Configuration

Set environment variables or use command-line arguments:

```bash
# Environment Variables
export MAPLE_HOST=127.0.0.1                    # Server host (default: 127.0.0.1)
export MAPLE_PORT=8080                         # Server port (default: 8080)
export MAPLE_BACKEND_URL=https://enclave.trymaple.ai   # Maple backend URL
export MAPLE_API_KEY=your-maple-api-key        # Default API key (optional)
export MAPLE_DEBUG=true                        # Enable debug logging
export MAPLE_ENABLE_CORS=true                  # Enable CORS
export MAPLE_REQUEST_TIMEOUT_SECS=300          # Backend request timeout
export MAPLE_STREAM_IDLE_TIMEOUT_SECS=300      # Streaming idle timeout between chunks
```

Or use CLI arguments:
```bash
cargo run -- --host 0.0.0.0 --port 8080 --backend-url https://enclave.trymaple.ai
```

For an unsigned local backend, use `just run-local`. That recipe alone enables
the explicitly named `insecure-local-mock-attestation` Cargo feature. Generic,
release, Docker, and embedded Maple builds leave the feature disabled.

## 🛠️ Usage

### Using as a Binary

#### Start the Server

```bash
cargo run
```

You should see:
```
🚀 Maple Proxy Server started successfully!
📋 Available endpoints:
   GET  /health              - Health check
   GET  /v1/models           - List available models
   POST /v1/chat/completions - Create chat completions (streaming & non-streaming)
   POST /v1/embeddings       - Create embeddings
```

### API Endpoints

#### List Models
```bash
curl http://localhost:8080/v1/models \
  -H "Authorization: Bearer YOUR_MAPLE_API_KEY"
```

#### Chat Completions
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

Set `stream` to `true` for Server-Sent Events or `false` for one JSON response.
Additional provider-specific JSON fields are forwarded without being parsed or
rewritten by the proxy or Rust SDK.

#### Embeddings
```bash
curl http://localhost:8080/v1/embeddings \
  -H "Authorization: Bearer YOUR_MAPLE_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "nomic-embed-text",
    "input": "Generate an embedding for this text"
  }'
```

### Using as a Library

You can also embed Maple Proxy in your own Rust application:

```rust
use maple_proxy::{Config, create_app};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create config programmatically
    let config = Config::new(
        "127.0.0.1".to_string(),
        8081,  // Custom port
        "https://enclave.trymaple.ai".to_string(),
    )
    .with_api_key("your-api-key-here".to_string())
    .with_debug(true)
    .with_cors(true);

    // Create the app
    let app = create_app(config.clone());

    // Start the server
    let addr = config.socket_addr()?;
    let listener = TcpListener::bind(addr).await?;
    
    println!("Maple proxy server running on http://{}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
```

Run the example:
```bash
cargo run --example library_usage
```

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

# Embeddings
curl http://localhost:8080/v1/embeddings \
  -H "Authorization: Bearer YOUR_MAPLE_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "nomic-embed-text",
    "input": "Generate an embedding for this text"
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

### Quick Start with Pre-built Image

Pull and run the official image from GitHub Container Registry:

```bash
# Pull the latest image
docker pull ghcr.io/opensecretcloud/maple-proxy:latest

# Run with your API key
docker run -p 8080:8080 \
  -e MAPLE_BACKEND_URL=https://enclave.trymaple.ai \
  -e MAPLE_REQUEST_TIMEOUT_SECS=300 \
  -e MAPLE_STREAM_IDLE_TIMEOUT_SECS=300 \
  ghcr.io/opensecretcloud/maple-proxy:latest
```

### Build from Source

```bash
# Build the image locally
just docker-build

# Run the container
just docker-run
```

### Production Docker Setup

1. **Option A: Use pre-built image from GHCR**
```bash
# In your docker-compose.yml, use:
image: ghcr.io/opensecretcloud/maple-proxy:latest
```

2. **Option B: Build your own image**
```bash
docker build -t maple-proxy:latest .
```

3. **Run with docker-compose:**
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
  - MAPLE_REQUEST_TIMEOUT_SECS=300                 # Backend request timeout
  - MAPLE_STREAM_IDLE_TIMEOUT_SECS=300             # Streaming idle timeout
  - RUST_LOG=info                                  # Logging level
  # - MAPLE_API_KEY=xxx                            # Only for private deployments!
```

## 🔧 Development

### Docker Images & CI/CD

**Automated Builds (GitHub Actions)**
- Every push to `master` automatically builds and publishes to `ghcr.io/opensecretcloud/maple-proxy:latest`
- Git tags (e.g., `v1.0.0`) trigger versioned releases
- Multi-platform images (linux/amd64, linux/arm64) built automatically
- No manual intervention needed - just push your code!

**Local Development (Justfile)**
```bash
# For local testing and debugging
just docker-build        # Build locally
just docker-run          # Test locally
just ghcr-push v1.2.3   # Manual push (requires login)
```

Use GitHub Actions for production releases, Justfile for local development.

### Build from Source
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
- `nomic-embed-text` - Embedding model for `/v1/embeddings`
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
2. **Maple Proxy** handles authentication and asks the OpenSecret SDK to
   establish the TEE channel
3. **OpenSecret SDK** authenticates and authorizes the enclave before accepting
   its key and completing key exchange
4. **Requests** are encrypted and forwarded to Maple's TEE infrastructure
5. **Responses** are streamed back to the client in OpenAI format

### TEE release authorization

The Sigstore/Rekor release-authorization work belongs in the OpenSecret SDK,
not in Maple Proxy. For each non-local backend, the SDK is expected to:

1. verify the AWS Nitro attestation document, certificate chain, nonce, and
   signature;
2. extract and validate the complete PCR0/PCR1/PCR2 measurement tuple;
3. compare that tuple with the release snapshot embedded in the SDK; and
4. accept the enclave public key and perform key exchange only after the tuple
   is present in that snapshot.

Maple Proxy continues to call `perform_attestation_handshake`; it neither
maintains a second PCR allowlist nor implements a separate Sigstore verifier.
Keeping this policy in the SDK gives every Rust SDK consumer the same
fail-closed authorization boundary before application data is sent.

There is no Sigstore, Rekor, or other release-metadata network lookup during a
runtime handshake. At SDK update time, the release-snapshot updater verifies
the release manifest and Cosign bundle, including the expected signing identity
and Rekor evidence, before generating the embedded snapshot. Consumers then
review and pin the SDK release containing that generated snapshot.

Sigstore makes a release statement and its signing identity tamper-evident in
an append-only transparency log. It does **not** prove that an artifact was
reproducibly built, and it does **not** make an old, previously authorized
release fresh. Reproducibility remains a separate Nix rebuild/compare property;
rollback prevention, revocation, or minimum-version policy must also be handled
separately.

> **Integration status:** this branch pins the exact reviewed SDK integration
> commit with default features disabled. Its embedded release snapshot is
> intentionally empty, so remote handshakes fail closed. Update the pin to a
> reviewed snapshot-bearing commit or published crate after the first signed
> backend release; do not merge or publish this staging state as a working
> production proxy.

## 📝 License

MIT License - see LICENSE file for details.

## 🤝 Contributing

Contributions welcome! Please feel free to submit a Pull Request.
