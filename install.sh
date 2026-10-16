#!/bin/bash

set -e

echo "==> Starting Apollo installation..."

# Check and install system dependencies
echo "==> Checking system dependencies (ALSA, compiler, git)..."
if command -v apt-get &> /dev/null; then
    echo "Detected Debian/Ubuntu-based system."
    sudo apt-get update
    sudo apt-get install -y libasound2-dev build-essential pkg-config curl git
elif command -v pacman &> /dev/null; then
    echo "Detected Arch Linux-based system."
    sudo pacman -Sy --needed alsa-lib base-devel curl git
elif command -v dnf &> /dev/null; then
    echo "Detected Fedora-based system."
    sudo dnf install -y alsa-lib-devel gcc pkgconf-pkg-config curl git
elif command -v zypper &> /dev/null; then
    echo "Detected openSUSE-based system."
    sudo zypper install -y alsa-devel gcc pkg-config curl git
else
    echo "Unrecognized package manager. Please ensure ALSA development libraries (e.g. libasound2-dev) and git are installed."
fi

# Clone repository and navigate into it
echo "==> Cloning Apollo Music Player repository..."
rm -rf apollo-music-player
git clone https://github.com/skinnzysan/apollo-music-player.git
cd apollo-music-player

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

# Install globally so 'apollo' can be launched from anywhere
echo "==> Installing the program..."
INSTALL_DIR="/usr/local/bin"
sudo mkdir -p "$INSTALL_DIR"
sudo cp target/release/apollo "$INSTALL_DIR/apollo"
sudo chmod +x "$INSTALL_DIR/apollo"

# Clean up cloned repository folder
echo "==> Cleaning up installation files..."
cd ..
rm -rf apollo-music-player

echo "==> Success! The program has been installed to $INSTALL_DIR/apollo."
echo "You can now start the player from anywhere by running:"
echo "apollo"
