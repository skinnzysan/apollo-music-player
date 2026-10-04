#!/bin/bash

set -e

echo "==> Rozpoczynam instalację Apollo..."

# Sprawdzanie i instalacja zależności systemowych
echo "==> Sprawdzanie zależności systemowych (ALSA, kompilator)..."
if command -v apt-get &> /dev/null; then
    echo "Wykryto system oparty na Debian/Ubuntu."
    sudo apt-get update
    sudo apt-get install -y libasound2-dev build-essential pkg-config curl
elif command -v pacman &> /dev/null; then
    echo "Wykryto system oparty na Arch Linux."
    sudo pacman -Sy --needed alsa-lib base-devel curl
elif command -v dnf &> /dev/null; then
    echo "Wykryto system oparty na Fedora."
    sudo dnf install -y alsa-lib-devel gcc pkgconf-pkg-config curl
elif command -v zypper &> /dev/null; then
    echo "Wykryto system oparty na openSUSE."
    sudo zypper install -y alsa-devel gcc pkg-config curl
else
    echo "Nie rozpoznano menedżera pakietów. Upewnij się, że biblioteki deweloperskie ALSA (np. libasound2-dev) są zainstalowane."
fi

# Sprawdzenie czy jest Rust/Cargo
echo "==> Sprawdzanie środowiska Rust..."
if ! command -v cargo &> /dev/null; then
    echo "Nie znaleziono Cargo. Rozpoczynam instalację języka Rust (rustup)..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust jest już zainstalowany."
fi

# Budowanie projektu
echo "==> Pobieranie bibliotek (Rust) i budowanie programu..."
cargo build --release

# Instalacja do katalogu użytkownika (żeby można było uruchamiać z dowolnego miejsca)
echo "==> Instalowanie programu..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

cp target/release/apollo "$INSTALL_DIR/apollo"
chmod +x "$INSTALL_DIR/apollo"

echo "==> Sukces! Program został zainstalowany w $INSTALL_DIR/apollo."

# Sprawdzenie czy INSTALL_DIR jest w PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "UWAGA: Katalog $INSTALL_DIR nie znajduje się w Twojej zmiennej PATH."
    echo "Aby móc odpalić program wpisując po prostu 'apollo' z dowolnego miejsca, dodaj tę linię do pliku ~/.bashrc lub ~/.zshrc:"
    echo 'export PATH="$HOME/.local/bin:$PATH"'
    echo "Następnie uruchom terminal ponownie lub wpisz 'source ~/.bashrc'."
else
    echo "Możesz teraz uruchomić odtwarzacz wpisując polecenie:"
    echo "apollo"
fi
