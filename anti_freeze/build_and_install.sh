#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

NAME="$(basename "$PWD")"

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --release -p app

pkill -x "$NAME" 2>/dev/null || true
pkill -x app 2>/dev/null || true
sleep 1

install -m 755 "target/release/$NAME" "$HOME/.local/bin/$NAME"
echo "installed $HOME/.local/bin/$NAME"