# Crosure — single entry point for Rust and TypeScript.

default:
    @just --list

# Run the desktop app in dev mode.
dev:
    cd tauri && npm install && npm run tauri dev

# Build the production bundle.
build:
    cd tauri && npm install && npm run tauri build

# All tests.
test:
    cargo test --workspace
    cd tauri && npm run test

# All linters.
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cd tauri && npm run lint

fmt:
    cargo fmt --all
    cd tauri && npm run format

fmt-check:
    cargo fmt --all -- --check
    cd tauri && npm run format:check
