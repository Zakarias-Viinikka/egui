#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --release -p app

install -m 755 target/release/app          "$HOME/.local/bin/zapcli"
install -m 755 target/release/zap-settings "$HOME/.local/bin/zap-settings"

cat > "$HOME/.local/bin/zap" << 'WRAP'
#!/usr/bin/env bash
exec "$HOME/.local/bin/zapcli" "$@"
WRAP
chmod +x "$HOME/.local/bin/zap"

echo "installed zapcli, zap-settings, and zap"
