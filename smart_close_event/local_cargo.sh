#!/usr/bin/env bash
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
exec cargo "${@:-run}"
