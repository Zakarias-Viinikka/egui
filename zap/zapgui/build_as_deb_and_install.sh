#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

NAME="zap"
PKG_NAME="zap"
VERSION="0.1.0"
ARCH="amd64"
TARGET_USER="$(id -un)"

command -v cargo    >/dev/null || { echo "cargo not found"; exit 1; }
command -v dpkg-deb >/dev/null || { echo "dpkg-deb not found (apt install dpkg-dev)"; exit 1; }

export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build --release -p app

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

ROOT="$STAGE/${NAME}_${VERSION}_${ARCH}"
mkdir -p "$ROOT/DEBIAN"
mkdir -p "$ROOT/usr/bin"

install -m 755 target/release/app "$ROOT/usr/bin/zapgui"

cat > "$ROOT/DEBIAN/control" <<CTRL
Package: $PKG_NAME
Version: $VERSION
Architecture: $ARCH
Maintainer: $TARGET_USER <$TARGET_USER@localhost>
Depends: libc6, libxcb1
Section: utils
Priority: optional
Description: zap - AI-proposed edit approval viewer
CTRL

OUT="$(pwd)/${PKG_NAME}_${VERSION}_${ARCH}.deb"
rm -f "$OUT"
sudo dpkg-deb --build --root-owner-group "$ROOT" "$OUT"
if [ -n "${SUDO_UID:-}" ]; then chown "$SUDO_UID:${SUDO_GID:-$SUDO_UID}" "$OUT"; fi

echo "built $OUT"
sudo dpkg -i "$OUT" || sudo apt-get -f install -y
echo "installed $OUT"
