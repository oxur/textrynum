# ECL Development Commands

# Default: show available commands
default:
    @just --list

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests with coverage (library code only)
coverage:
    cargo llvm-cov --workspace --lib --all-features --html
    @echo "Coverage report: target/llvm-cov/html/index.html"

# Generate coverage and check threshold (≥95%)
coverage-check:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Generating coverage report..."
    cargo llvm-cov --workspace --lib --all-features --html --quiet
    echo ""
    echo "Coverage Summary:"
    cargo llvm-cov --workspace --lib --all-features --summary-only
    echo ""
    COVERAGE=$(cargo llvm-cov --workspace --lib --all-features --summary-only | grep -oP '\d+\.\d+(?=%)' | head -1 || echo "0")
    echo "Line coverage: $COVERAGE%"
    if (( $(echo "$COVERAGE < 95" | bc -l) )); then
        echo "❌ Coverage $COVERAGE% is below 95% threshold"
        exit 1
    else
        echo "✅ Coverage $COVERAGE% meets 95% threshold"
    fi

# Check code quality
lint:
    cargo clippy --workspace --all-features --all-targets -- -D warnings
    cargo fmt --all -- --check

# Format code
format:
    cargo fmt --all

# Build all crates
build:
    cargo build --workspace --all-features

# Build for release
build-release:
    cargo build --workspace --all-features --release

# Run development environment (Restate + app)
dev:
    docker compose up -d restate
    cargo run --bin ecl-workflows

# Stop development environment
dev-stop:
    docker compose down

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/

# Check for dependency updates
outdated:
    cargo outdated --workspace

# Run security audit
audit:
    cargo audit

# Generate documentation
docs:
    cargo doc --workspace --all-features --no-deps --open
