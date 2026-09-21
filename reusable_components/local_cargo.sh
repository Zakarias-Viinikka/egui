#!/usr/bin/env bash
# Always build from this workspace using this script, never bare `cargo`.
#
# Why: this user's global cargo config (~/.cargo/config.toml) sets
#   rustflags = ["-Zcodegen-backend=cranelift", ...]
# cranelift cannot compile the float-to-int instruction that fontdue uses
# for text rendering. Any crate in this workspace that depends on fontdue
# (currently: fading_popup) will panic at runtime with
#   "llvm.x86.sse.cvtss2si is not yet supported"
# if built with cranelift.
#
# Setting RUSTFLAGS in the environment overrides the config's rustflags
# for this invocation, so cranelift doesn't get passed to rustc.
#
# Usage:
#   ./local_cargo.sh                   # build the workspace
#   ./local_cargo.sh check -p fading_popup
#   ./local_cargo.sh build --release
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
exec cargo "$@"
