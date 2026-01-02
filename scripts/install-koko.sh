#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

echo "=== kokorox installer ==="
echo ""
echo "Detected platform: ${OS} (${ARCH})"
echo ""

# GPU acceleration options
echo "Select GPU acceleration (improves TTS inference performance):"
echo ""
echo "  1) None (CPU only) - works everywhere, slower"
echo "  2) CUDA (NVIDIA GPU) - requires CUDA toolkit installed"
echo "  3) CoreML (Apple Silicon) - macOS only, uses Neural Engine"
echo ""

# Default based on platform
if [[ "${OS}" == "Darwin" && "${ARCH}" == "arm64" ]]; then
    DEFAULT_CHOICE="3"
    echo "Recommended for Apple Silicon: CoreML (3)"
elif [[ "${OS}" == "Linux" ]] && command -v nvidia-smi &> /dev/null; then
    DEFAULT_CHOICE="2"
    echo "NVIDIA GPU detected, recommended: CUDA (2)"
else
    DEFAULT_CHOICE="1"
    echo "Recommended: CPU only (1)"
fi

echo ""
read -p "Enter choice [${DEFAULT_CHOICE}]: " CHOICE
CHOICE="${CHOICE:-$DEFAULT_CHOICE}"

# Set features based on choice
FEATURES=""
case "${CHOICE}" in
    1)
        echo "Building with CPU-only support..."
        # No feature flag needed - CPU is default
        ;;
    2)
        echo "Building with CUDA support..."
        FEATURES="cuda"
        ;;
    3)
        if [[ "${OS}" != "Darwin" ]]; then
            echo "Warning: CoreML is only available on macOS. Falling back to CPU."
            # No feature flag needed - CPU is default
        else
            echo "Building with CoreML support..."
            FEATURES="coreml"
        fi
        ;;
    *)
        echo "Invalid choice. Using CPU-only."
        # No feature flag needed - CPU is default
        ;;
esac

echo ""

# Installation location
echo "Select installation location:"
echo ""
echo "  1) ~/.cargo/bin (requires cargo, standard for Rust tools)"
echo "  2) ~/.local/bin (no cargo needed, add to PATH if not already)"
echo "  3) /usr/local/bin (system-wide, requires sudo)"
echo ""

read -p "Enter choice [1]: " INSTALL_CHOICE
INSTALL_CHOICE="${INSTALL_CHOICE:-1}"

echo ""
if [[ -n "${FEATURES}" ]]; then
    echo "Building koko with features: ${FEATURES}..."
else
    echo "Building koko (CPU only)..."
fi
echo ""

cd "${ROOT_DIR}"

# Build feature flags
FEATURE_FLAGS=""
if [[ -n "${FEATURES}" ]]; then
    FEATURE_FLAGS="--features ${FEATURES}"
fi

case "${INSTALL_CHOICE}" in
    1)
        echo "Installing to ~/.cargo/bin..."
        if [[ -n "${FEATURES}" ]]; then
            cargo install --path koko --features "${FEATURES}" --force
        else
            cargo install --path koko --force
        fi
        INSTALL_PATH="$HOME/.cargo/bin/koko"
        ;;
    2)
        echo "Building release binary..."
        if [[ -n "${FEATURES}" ]]; then
            cargo build --release --package koko --features "${FEATURES}"
        else
            cargo build --release --package koko
        fi
        mkdir -p ~/.local/bin
        cp target/release/koko ~/.local/bin/
        INSTALL_PATH="$HOME/.local/bin/koko"
        echo "Installed to ~/.local/bin/koko"
        
        # Check if ~/.local/bin is in PATH
        if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
            echo ""
            echo "Warning: ~/.local/bin is not in your PATH."
            echo "Add this to your shell config (~/.bashrc, ~/.zshrc, etc.):"
            echo '  export PATH="$HOME/.local/bin:$PATH"'
        fi
        ;;
    3)
        echo "Building release binary..."
        if [[ -n "${FEATURES}" ]]; then
            cargo build --release --package koko --features "${FEATURES}"
        else
            cargo build --release --package koko
        fi
        echo "Installing to /usr/local/bin (requires sudo)..."
        sudo cp target/release/koko /usr/local/bin/
        INSTALL_PATH="/usr/local/bin/koko"
        echo "Installed to /usr/local/bin/koko"
        ;;
    *)
        echo "Invalid choice. Installing to ~/.cargo/bin..."
        if [[ -n "${FEATURES}" ]]; then
            cargo install --path koko --features "${FEATURES}" --force
        else
            cargo install --path koko --force
        fi
        INSTALL_PATH="$HOME/.cargo/bin/koko"
        ;;
esac

# Setup voice data
VOICES_SRC="${ROOT_DIR}/data/voices-v1.0.bin"
VOICES_DEST="$HOME/.cache/kokoros/data.voices-v1.0.bin"

if [[ -f "${VOICES_SRC}" ]]; then
    echo ""
    echo "Copying voice data to cache..."
    mkdir -p "$(dirname "${VOICES_DEST}")"
    cp "${VOICES_SRC}" "${VOICES_DEST}"
fi

echo ""
echo "=== Installation complete! ==="
echo ""
echo "Binary: ${INSTALL_PATH}"
echo "GPU:    ${FEATURES}"
echo ""
echo "Quick start:"
echo "  koko \"Hello, world!\"              # Speak text"
echo "  koko \"Hello\" -o hello.wav         # Save to file"
echo "  koko \"Hello\" --voice af_heart     # Use specific voice"
echo "  koko --list-voices                 # List available voices"
echo ""
echo "Download models (if not already cached):"
echo "  koko --download                    # Download default model"
echo ""
