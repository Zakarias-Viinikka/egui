#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

PKG_NAME="chat"
VERSION="0.1.0"
BIN_NAME="chat"
ARCH="amd64"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

export RUSTFLAGS="-C link-arg=-fuse-ld=mold -Zcodegen-backend=llvm"
cargo build --release -p app

mkdir -p "$STAGE/DEBIAN"
mkdir -p "$STAGE/usr/bin"

cat > "$STAGE/DEBIAN/control" <<CTRL
Package: $PKG_NAME
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Maintainer: zakke
Description: twitch chat overlay
CTRL

install -m 755 "target/release/$BIN_NAME" "$STAGE/usr/bin/$PKG_NAME"

OUT="$PWD/target/${PKG_NAME}_${VERSION}_${ARCH}.deb"
dpkg-deb --build --root-owner-group "$STAGE" "$OUT"

sudo dpkg -i "$OUT"
