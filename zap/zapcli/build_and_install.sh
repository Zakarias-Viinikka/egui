#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --release -p app

install -m 755 target/release/app          "$HOME/.local/bin/zapcli"
install -m 755 target/release/zap-settings "$HOME/.local/bin/zap-settings"

cat > "$HOME/.local/bin/zap" << 'WRAP'
#!/usr/bin/env bash
if [ "${1:-}" = "p" ]; then
    sock="/run/user/1000/zapcli.sock"
    if [ ! -S "$sock" ]; then
        setsid "$HOME/.local/bin/zapcli" >/dev/null 2>&1 < /dev/null &
        for _ in $(seq 1 60); do
            [ -S "$sock" ] && break
            sleep 0.05
        done
    fi
    python3 - "$sock" "$PWD" << 'PYEOF' >/dev/null 2>&1
import socket, sys
sock, path = sys.argv[1], sys.argv[2]
try:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(sock)
    s.sendall(("cd " + path).encode())
    s.close()
except Exception:
    sys.exit(1)
PYEOF
    exit 0
fi
setsid "$HOME/.local/bin/zapcli" "$@" >/dev/null 2>&1 < /dev/null &
WRAP
chmod +x "$HOME/.local/bin/zap"

pkill -x zapcli 2>/dev/null || true
pkill -x zapgui 2>/dev/null || true
echo "installed zapcli, zap-settings, and zap"
