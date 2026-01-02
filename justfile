# kokorox - Kokoro TTS in Rust
# https://github.com/byteowlz/kokorox

set positional-arguments

# === Default ===

# List available commands
default:
    @just --list

# === Build ===

# Build debug binaries
build:
    cargo build

# Build release binaries
build-release:
    cargo build --release

# Fast compile check
check:
    cargo check --workspace

# === Test ===

# Run tests
test:
    cargo nextest run --no-fail-fast

# Run tests with cargo test (fallback)
test-cargo:
    cargo test

# === Lint & Format ===

# Run clippy linter
clippy:
    cargo clippy --all-features --tests -- -D warnings

# Alias for clippy
lint: clippy

# Auto-fix lint warnings
fix *args:
    cargo clippy --fix --all-features --tests --allow-dirty "$@"

# Format code
fmt:
    cargo fmt -- --config imports_granularity=Item

# Check formatting
fmt-check:
    cargo fmt -- --check

# === Install ===

# Interactive install with GPU acceleration selection
install-all:
    ./scripts/install-koko.sh

# Install koko CLI to ~/.cargo/bin (CPU only, default)
install:
    cargo install --path koko --force

# Install koko CLI with CUDA support
install-cuda:
    cargo install --path koko --features cuda --force

# Install koko CLI with CoreML support (macOS only)
install-coreml:
    cargo install --path koko --features coreml --force

# Install to ~/.local/bin (for non-cargo users)
install-local:
    cargo build --release --package koko
    mkdir -p ~/.local/bin
    cp target/release/koko ~/.local/bin/
    @echo "Installed koko to ~/.local/bin/koko"

# Uninstall from ~/.cargo/bin
uninstall:
    cargo uninstall koko || true

# Uninstall from ~/.local/bin
uninstall-local:
    rm -f ~/.local/bin/koko
    @echo "Removed koko from ~/.local/bin"

# === Docs ===

# Generate documentation
docs:
    cargo doc --no-deps --workspace --open

# === Clean ===

# Clean build artifacts
clean:
    cargo clean

# === Development ===

# Run koko CLI
run *args:
    cargo run --package koko -- {{args}}

# Run koko CLI (release mode)
run-release *args:
    cargo run --package koko --release -- {{args}}

# Watch for changes and rebuild
watch:
    cargo watch -x check

# === Dependencies ===

# Fetch dependencies
fetch:
    rustup show active-toolchain
    cargo fetch

# Update dependencies
update:
    cargo update

# === Release ===

# Bump version and release (requires cargo-release)
release *args:
    cargo release {{args}}
