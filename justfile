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

# Browser preview without the desktop shell: dev bridge on :1421 + Vite on :1420.
preview:
    cargo build -p crosure --features devbridge --bin crosure-devbridge
    ./target/debug/crosure-devbridge & cd tauri && npm install && npx vite --port 1420

# Rebuild the benign test fixtures (needs cc, strip, x86_64-w64-mingw32-gcc).
fixtures:
    crates/crosure-engine/tests/fixtures/build.sh
