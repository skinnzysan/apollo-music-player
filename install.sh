#!/bin/bash

set -e

echo "==> Starting Apollo installation..."

# Check and install system dependencies
echo "==> Checking system dependencies (ALSA, compiler)..."
if command -v apt-get &> /dev/null; then
    echo "Detected Debian/Ubuntu-based system."
    sudo apt-get update
    sudo apt-get install -y libasound2-dev build-essential pkg-config curl
elif command -v pacman &> /dev/null; then
    echo "Detected Arch Linux-based system."
    sudo pacman -Sy --needed alsa-lib base-devel curl
elif command -v dnf &> /dev/null; then
    echo "Detected Fedora-based system."
    sudo dnf install -y alsa-lib-devel gcc pkgconf-pkg-config curl
elif command -v zypper &> /dev/null; then
    echo "Detected openSUSE-based system."
    sudo zypper install -y alsa-devel gcc pkg-config curl
else
    echo "Unrecognized package manager. Please ensure ALSA development libraries (e.g. libasound2-dev) are installed."
fi

# Check for Rust/Cargo
echo "==> Checking Rust environment..."
if ! command -v cargo &> /dev/null; then
    echo "Cargo not found. Starting Rust installation (rustup)..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed."
fi

# Build project
echo "==> Fetching libraries (Rust) and building the program..."
cargo build --release

# Install to user directory
echo "==> Installing the program..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

cp target/release/apollo "$INSTALL_DIR/apollo"
chmod +x "$INSTALL_DIR/apollo"

echo "==> Success! The program has been installed to $INSTALL_DIR/apollo."

# Check if INSTALL_DIR is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "WARNING: The directory $INSTALL_DIR is not in your PATH."
    echo "To run the program by just typing 'apollo' from anywhere, add this line to your ~/.bashrc or ~/.zshrc:"
    echo 'export PATH="$HOME/.local/bin:$PATH"'
    echo "Then restart your terminal or run 'source ~/.bashrc'."
else
    echo "You can now start the player by running:"
    echo "apollo"
fi
