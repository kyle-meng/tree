#!/bin/bash
set -e

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
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

echo "Downloading $BINARY..."
curl -L "$BASE_URL/$BINARY" -o tree
chmod +x tree
sudo mv tree /usr/local/bin/
echo "Installation complete!"