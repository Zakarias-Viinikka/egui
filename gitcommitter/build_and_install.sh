#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --release -p app
install -m 755 "target/release/app" "$HOME/.local/bin/gitcommitter"
echo "installed $HOME/.local/bin/gitcommitter"
