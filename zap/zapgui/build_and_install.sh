#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --release -p app
install -m 755 "target/release/app" "$HOME/.local/bin/zapgui"
pkill -x zapcli 2>/dev/null || true
pkill -x zapgui 2>/dev/null || true
echo "installed $HOME/.local/bin/zapgui"
