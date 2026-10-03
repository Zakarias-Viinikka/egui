#!/usr/bin/env bash
set -euo pipefail

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

cd "$HOME/ProgStuff/egui/zap"
cargo build --release -p app
install -m 755 target/release/app "$HOME/.local/bin/zapgui"

cd "$HOME/ProgStuff/egui/zapcli"
cargo build --release -p app
install -m 755 target/release/app          "$HOME/.local/bin/zapcli"
install -m 755 target/release/zap-settings "$HOME/.local/bin/zap-settings"

cat > "$HOME/.local/bin/zap" << 'WRAP'
#!/usr/bin/env bash
exec "$HOME/.local/bin/zapcli" "$@"
WRAP
chmod +x "$HOME/.local/bin/zap"

echo "installed zapgui, zapcli, zap-settings, and zap"
