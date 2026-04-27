#!/usr/bin/env bash
# VOCA — Whisper model downloader
# Usage: bash scripts/download_model.sh [tiny|base|small|medium]
# Default: base

set -euo pipefail

MODELS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/models"
BASE_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main"

declare -A MODELS=(
  [tiny]="ggml-tiny.bin"
  [base]="ggml-base.bin"
  [small]="ggml-small.bin"
  [medium]="ggml-medium.bin"
)

declare -A SHA256=(
  [tiny]="bd577a113a864445d4c299885e0cb97d4ba92b5f"
  [base]="465707469ff3a37a2b9b8d8f89f2f99de7299dac"
  [small]="1be3a9b2063867b937e64e2ec7483364a79917e57cf44de5756a4a89a0ad3e9e"
  [medium]="fd9727b6e1217c2f614f9b698455c4ffd82463b4"
)

TIER="${1:-base}"

if [[ -z "${MODELS[$TIER]+_}" ]]; then
  echo "Unknown model tier: $TIER"
  echo "Available: tiny, base, small, medium"
  exit 1
fi

FILENAME="${MODELS[$TIER]}"
DEST="$MODELS_DIR/$FILENAME"

if [[ -f "$DEST" ]]; then
  echo "✓ $FILENAME already exists at $DEST"
  exit 0
fi

mkdir -p "$MODELS_DIR"

echo "Downloading $FILENAME (~$(
  case $TIER in
    tiny) echo "75 MB" ;;
    base) echo "142 MB" ;;
    small) echo "466 MB" ;;
    medium) echo "1.5 GB" ;;
  esac
))..."

URL="$BASE_URL/$FILENAME"

if command -v curl &>/dev/null; then
  curl -L --progress-bar -o "$DEST" "$URL"
elif command -v wget &>/dev/null; then
  wget --show-progress -q -O "$DEST" "$URL"
else
  echo "Error: curl or wget is required"
  exit 1
fi

echo "✓ Downloaded to $DEST"
echo "Set model_path = \"$DEST\" in ~/.config/voca/config.toml"
