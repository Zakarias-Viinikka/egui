#!/usr/bin/env bash
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
if [ $# -eq 0 ]; then
    exec cargo run -p popup_host_app
fi
exec cargo "$@"
