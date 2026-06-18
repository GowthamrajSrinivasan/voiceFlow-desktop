#!/usr/bin/env bash
set -e

# Define directories
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODELS_DIR="$ROOT_DIR/apps/tauri-app/src-tauri/models"

# Create models directory if it doesn't exist
mkdir -p "$MODELS_DIR"

# Download Whisper Base EN if missing
MODEL_PATH="$MODELS_DIR/ggml-base.en.bin"
if [ ! -f "$MODEL_PATH" ]; then
    echo "Downloading Whisper Base EN model..."
    curl -L "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin" -o "$MODEL_PATH"
else
    echo "Whisper Base EN model already exists. Skipping."
fi

echo "Models fetched successfully."
