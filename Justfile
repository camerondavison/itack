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
check: check-plugin
    just --fmt --unstable --check
    cargo fmt --check
    cargo clippy -- -D warnings

check-plugin: check-plugin-manifest
    claude plugin validate .
    claude plugin validate ./plugins/    

check-plugin-manifest:
    #!/usr/bin/env bash
    set -euxo pipefail
    uv run --with pyyaml python <(curl -s https://raw.githubusercontent.com/anthropics/skills/69c0b1a0674149f27b61b2635f935524b6add202/skills/skill-creator/scripts/quick_validate.py) plugins/skills/itack-basics/

# Install the binary locally
install:
    cargo install --path .
