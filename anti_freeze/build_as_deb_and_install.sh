#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

NAME="anti_freeze"
PKG_NAME="anti-freeze"
VERSION="0.1.0"
ARCH="amd64"
TARGET_USER="$(id -un)"
TARGET_HOME="$HOME"

command -v cargo       >/dev/null || { echo "cargo not found"; exit 1; }
command -v dpkg-deb    >/dev/null || { echo "dpkg-deb not found (apt install dpkg-dev)"; exit 1; }
[ -x /usr/local/bin/anti_freeze_cap ] || { echo "missing helper: /usr/local/bin/anti_freeze_cap"; exit 1; }
[ -f /etc/sudoers.d/anti_freeze_cap ] || { echo "missing sudoers rule: /etc/sudoers.d/anti_freeze_cap"; exit 1; }

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --release -p app -p fading_popup -p watchdog

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

ROOT="$STAGE/${NAME}_${VERSION}_${ARCH}"
mkdir -p "$ROOT/DEBIAN"
mkdir -p "$ROOT/usr/bin"
mkdir -p "$ROOT/usr/local/bin"
mkdir -p "$ROOT/etc/sudoers.d"

install -m 755 target/release/app          "$ROOT/usr/bin/$NAME"
install -m 755 target/release/fading_popup "$ROOT/usr/bin/fading_popup"
install -m 755 target/release/watchdog     "$ROOT/usr/bin/watchdog"
install -m 755 /usr/local/bin/anti_freeze_cap "$ROOT/usr/local/bin/anti_freeze_cap"
sudo install -m 0440 /etc/sudoers.d/anti_freeze_cap "$ROOT/etc/sudoers.d/anti_freeze_cap"

cat > "$ROOT/DEBIAN/control" <<EOF
Package: $PKG_NAME
Version: $VERSION
Architecture: $ARCH
Maintainer: $TARGET_USER <$TARGET_USER@localhost>
Depends: libc6, libxcb1
Section: utils
Priority: optional
Description: anti_freeze - fullscreen RAM-hog killer with freeze and RAM cap
EOF

cat > "$ROOT/DEBIAN/postinst" <<EOF
#!/bin/sh
set -e
USER_NAME="$TARGET_USER"
HOME_DIR="$TARGET_HOME"
DESKTOP="\$HOME_DIR/.config/autostart/anti_freeze.desktop"

mkdir -p "\$HOME_DIR/.config/autostart"

if [ ! -f "\$DESKTOP" ]; then
    cat > "\$DESKTOP" <<DESK
[Desktop Entry]
Type=Application
Name=anti_freeze
Comment=RAM-hog killer with freeze and RAM cap
Exec=/usr/bin/anti_freeze
Terminal=false
X-GNOME-Autostart-enabled=true
DESK
    chown "\$USER_NAME:\$USER_NAME" "\$DESKTOP" 2>/dev/null || true
    echo "added autostart: \$DESKTOP"
else
    echo "autostart already present: \$DESKTOP"
fi
EOF
chmod 755 "$ROOT/DEBIAN/postinst"

OUT="$(pwd)/${PKG_NAME}_${VERSION}_${ARCH}.deb"
rm -f "$OUT"
sudo dpkg-deb --build --root-owner-group "$ROOT" "$OUT"
if [ -n "${SUDO_UID:-}" ]; then chown "$SUDO_UID:${SUDO_GID:-$SUDO_UID}" "$OUT"; fi

echo "built $OUT"
sudo dpkg -i "$OUT" || sudo apt-get -f install -y
echo "installed $OUT"
