#!/usr/bin/env bash
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
case "${1:-run}" in
  run|r)
    release=()
    for a in "$@"; do
      if [ "$a" = "--release" ]; then release=(--release); fi
    done
    cargo build -p watchdog "${release[@]}" || exit 1
    ;;
esac
exec cargo "${@:-run}"
