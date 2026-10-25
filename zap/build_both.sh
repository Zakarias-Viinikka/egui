#!/usr/bin/env bash
set -euo pipefail

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

cd "$HOME/ProgStuff/egui/zap/zapgui"
cargo build --release -p app
install -m 755 target/release/app "$HOME/.local/bin/zapgui"

cd "$HOME/ProgStuff/egui/zap/zapcli"
cargo build --release -p app
install -m 755 target/release/app          "$HOME/.local/bin/zapcli"
install -m 755 target/release/zap-settings "$HOME/.local/bin/zap-settings"

cat > "$HOME/.local/bin/zap" << 'WRAP'
#!/usr/bin/env bash
if [ "$1" = "-free" ]; then
    shift
    setsid "$HOME/.local/bin/zapcli" "$@" >/dev/null 2>&1 < /dev/null &
    exit 0
fi
exec "$HOME/.local/bin/zapcli" "$@"
WRAP
chmod +x "$HOME/.local/bin/zap"

cd "$HOME/ProgStuff/egui/zap/zapgui/fading_popup"
[ -f fonts/Ubuntu-Light.ttf ] || {
    mkdir -p fonts
    cp "$HOME/ProgStuff/egui/reusable_components/fading_popup/fonts/Ubuntu-Light.ttf" fonts/
}
cargo build --release
install -m 755 target/release/fading_popup "$HOME/.local/bin/fading_popup"

pkill -x zapgui 2>/dev/null || true
nohup "$HOME/.local/bin/zapgui" --hidden >/dev/null 2>&1 &

echo "installed zapgui, zapcli, zap-settings, and zap"
