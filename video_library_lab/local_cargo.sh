#!/usr/bin/env bash
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cmd="${1:-run}"
if [ "$cmd" = "run" ]; then
    cargo build --workspace --bins || exit $?
    exec cargo run "$@"
fi
exec cargo "${@:-run}"