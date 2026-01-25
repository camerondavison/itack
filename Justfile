# Default recipe - show available commands
default:
    just --list

# Run tests
test:
    cargo test

# Format and auto-fix code
fix:
    just --fmt --unstable
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged

# Verify code quality without modifications
check:
    just --fmt --unstable --check
    cargo fmt --check
    cargo clippy -- -D warnings

# Install the binary locally
install:
    cargo install --path .
