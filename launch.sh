#!/usr/bin/env bash
set -e

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64)  BIN="cosmic-sim-macos-arm64" ;;
      x86_64) BIN="cosmic-sim-macos-x86_64" ;;
      *) echo "Unsupported macOS architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64) BIN="cosmic-sim-linux-x86_64" ;;
      *) echo "Unsupported Linux architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"
    echo "On Windows, run cosmic-sim-windows-x86_64.exe directly."
    exit 1
    ;;
esac

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET="$SCRIPT_DIR/$BIN"

if [ ! -f "$TARGET" ]; then
  echo "Binary not found: $TARGET"
  echo "Download $BIN from the release page and place it in the same directory as this script."
  exit 1
fi

chmod +x "$TARGET"

if [ "$OS" = "Darwin" ]; then
  xattr -d com.apple.quarantine "$TARGET" 2>/dev/null || true
fi

exec "$TARGET" "$@"
