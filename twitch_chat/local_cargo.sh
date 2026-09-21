#!/usr/bin/env bash
export RUSTFLAGS="-C link-arg=-fuse-ld=mold -Zcodegen-backend=llvm"
cargo "${@:-run}"
