#!/usr/bin/env bash
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

first="${1:-run}"
case "$first" in
  r|run)
    cargo build --bins || exit 1
    ;;
esac

exec cargo "${@:-run}"
