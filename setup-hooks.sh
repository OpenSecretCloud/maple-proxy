#!/usr/bin/env bash
# Setup git hooks for Maple Proxy

echo "🔗 Setting up git hooks..."

# Get the git directory
GIT_DIR=$(git rev-parse --git-dir 2>/dev/null)

if [ -z "$GIT_DIR" ]; then
    echo "❌ Not in a git repository"
    exit 1
fi

# Set git hooks path to use our custom hooks
git config core.hooksPath .githooks

# Make sure hooks are executable
chmod +x .githooks/*

echo "✅ Git hooks installed successfully!"
echo "📝 Pre-commit hook will run:"
echo "   - cargo fmt --check"
echo "   - cargo clippy"
echo "   - cargo check"
echo "   - cargo test"
echo ""
echo "To bypass hooks (not recommended), use: git commit --no-verify"