#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

echo "=== [1/5] Installing system dependencies ==="
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  librsvg2-dev \
  patchelf \
  libayatana-appindicator3-dev \
  libsoup-3.0-dev \
  javascriptcoregtk-4.1-dev

echo "=== [2/5] Checking and installing Rust toolchain ==="
if ! command -v rustc &> /dev/null; then
    echo "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi
source "$HOME/.cargo/env"

echo "=== [3/5] Verifying Node.js and npm (v18+) ==="
if ! command -v node &> /dev/null || ! command -v npm &> /dev/null; then
    echo "Error: Node.js and npm are required but not installed. Please install them (e.g., via nvm or mise) and re-run." >&2
    exit 1
fi
node --version && npm --version

echo "=== [4/5] Building the Svelte renderer ==="
if [ -d "ui" ]; then
    cd ui
    npm install
    npm run build
    cd ..
else
    echo "Error: 'ui' directory not found in the current working directory." >&2
    exit 1
fi

echo "=== [5/5] Building the Tauri release app via Cargo ==="
cargo build -p sop-app --release

echo "=== Build completed successfully! Binary is located in target/release/ ==="
