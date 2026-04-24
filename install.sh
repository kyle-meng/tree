#!/bin/bash
set -e

OS="$(uname -s | tr 'A-Z' 'a-z')"
ARCH="$(uname -m)"
VERSION="v0.2.1"
BASE_URL="https://github.com/kyle-meng/tree/releases/download/$VERSION"

case "$OS" in
  linux)
    if [ "$ARCH" = "x86_64" ]; then
      BINARY="tree-x86_64-unknown-linux-musl"
    elif [ "$ARCH" = "aarch64" ]; then
      BINARY="tree-aarch64-unknown-linux-musl"
    fi
    ;;
  darwin)
    if [ "$ARCH" = "arm64" ]; then
      BINARY="tree-aarch64-apple-darwin"
    fi
    ;;
  msys*|mingw*|cygwin*)
    BINARY="tree-x86_64-pc-windows-gnu.exe"
    ;;
esac

if [ -z "$BINARY" ]; then
  echo "Unsupported OS or Architecture: $OS / $ARCH"
  exit 1
fi

INSTALL_DIR="/usr/local/bin"
# 如果 /usr/local/bin 不存在，则使用 /usr/bin
if [ ! -d "$INSTALL_DIR" ]; then
  INSTALL_DIR="/usr/bin"
fi

echo "Downloading $BINARY..."
curl -L "$BASE_URL/$BINARY" -o tree
chmod +x tree

if [ "$(id -u)" -eq 0 ]; then
  mv tree "$INSTALL_DIR/"
else
  if command -v sudo >/dev/null 2>&1; then
    sudo mv tree "$INSTALL_DIR/"
  else
    echo "Error: Need root privileges to install to $INSTALL_DIR. Please run as root or install 'sudo'."
    exit 1
  fi
fi

echo "Installation complete! Binary installed to $INSTALL_DIR/tree"