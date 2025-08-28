# Justfile for Maple Proxy
# Development commands for the OpenAI-compatible proxy server

# Default command - show available commands
default:
    @just --list

# Set up development environment
setup:
    @echo "🔧 Setting up development environment..."
    @bash setup-hooks.sh
    @cargo check --all-features
    @echo "✅ Development environment ready"

# Format code with rustfmt
format:
    @echo "🎨 Formatting code..."
    @cargo fmt
    @echo "✅ Code formatted"

# Alias for format
fmt: format

# Run clippy lints
lint:
    @echo "🔍 Running clippy lints..."
    @cargo clippy --all-targets --all-features -- -D warnings
    @echo "✅ Lints passed"

# Alias for lint
clippy: lint

# Run all tests
test:
    @echo "🧪 Running tests..."
    @cargo test --all-features
    @echo "✅ Tests passed"

# Run all checks (format, lint, test)
check: format lint test
    @echo "✅ All checks passed"

# Run the development server
run:
    @echo "🚀 Starting Maple Proxy server..."
    @echo "📝 Loading configuration from .env"
    @cargo run

# Run with custom backend URL (preserves .env variables)
run-with-backend url:
    @echo "🚀 Starting Maple Proxy server..."
    @echo "🔗 Backend: {{url}}"
    @echo "📝 Loading other configs from .env"
    @bash -c 'set -a; source .env 2>/dev/null; set +a; MAPLE_BACKEND_URL={{url}} cargo run'

# Run pointing to local backend
run-local:
    @just run-with-backend "http://localhost:3000"

# Run pointing to production backend
run-prod:
    @just run-with-backend "https://enclave.trymaple.ai"

# Build debug binary
build:
    @echo "🔨 Building debug binary..."
    @cargo build
    @echo "✅ Debug binary built at target/debug/maple-proxy"

# Build release binary
release:
    @echo "📦 Building release binary..."
    @cargo build --release
    @echo "✅ Release binary built at target/release/maple-proxy"

# Build for all targets (used in CI)
build-all:
    @echo "📦 Building for all targets..."
    @cargo build --all-targets --all-features
    @echo "✅ All targets built"

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    @cargo clean
    @echo "✅ Build artifacts cleaned"

# Update dependencies
update:
    @echo "📦 Updating dependencies..."
    @cargo update
    @echo "✅ Dependencies updated"

# Install to ~/.cargo/bin
install:
    @echo "📥 Installing maple-proxy..."
    @cargo install --path .
    @echo "✅ Installed to ~/.cargo/bin/maple-proxy"

# Uninstall from ~/.cargo/bin
uninstall:
    @echo "📤 Uninstalling maple-proxy..."
    @cargo uninstall maple-proxy
    @echo "✅ Uninstalled"

# Run with verbose logging
debug:
    @echo "🐛 Starting with debug logging..."
    RUST_LOG=debug MAPLE_DEBUG=true cargo run

# Check for security vulnerabilities
audit:
    @echo "🔒 Running security audit..."
    @cargo audit
    @echo "✅ Security audit complete"

# Generate documentation
doc:
    @echo "📚 Generating documentation..."
    @cargo doc --no-deps --all-features --open
    @echo "✅ Documentation generated"

# Show code coverage (requires cargo-tarpaulin)
coverage:
    @echo "📊 Generating code coverage..."
    @cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out html
    @echo "✅ Coverage report generated at tarpaulin-report.html"

# Watch for changes and run tests
watch:
    @echo "👁️ Watching for changes..."
    @cargo watch -x test

# Create a new git commit with conventional commit message
commit message:
    @git add -A
    @git commit -m "{{message}}"
    @echo "✅ Changes committed"

# Quick test with curl
test-curl:
    @echo "🧪 Testing with curl..."
    @curl -N http://localhost:8080/v1/chat/completions \
        -H "Content-Type: application/json" \
        -d '{"model": "llama3-3-70b", "messages": [{"role": "user", "content": "Say hello"}], "stream": true}'

# Show environment info
env:
    @echo "🌍 Environment Info"
    @echo "==================="
    @echo "Rust version: $(rustc --version)"
    @echo "Cargo version: $(cargo --version)"
    @echo "Just version: $(just --version)"
    @echo ""
    @echo "📋 Environment Variables:"
    @echo "MAPLE_HOST: ${MAPLE_HOST:-127.0.0.1}"
    @echo "MAPLE_PORT: ${MAPLE_PORT:-8080}"
    @echo "MAPLE_BACKEND_URL: ${MAPLE_BACKEND_URL:-https://enclave.trymaple.ai}"
    @echo "MAPLE_API_KEY: ${MAPLE_API_KEY:-[not set]}"
    @echo "MAPLE_DEBUG: ${MAPLE_DEBUG:-false}"
    @echo "MAPLE_ENABLE_CORS: ${MAPLE_ENABLE_CORS:-false}"
