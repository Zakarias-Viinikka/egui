#!/usr/bin/env bash
set -euo pipefail

# The user's global ~/.cargo/config.toml forces cranelift, which can't
# compile fontdue and other SSE intrinsics. RUSTFLAGS overrides the
# config's rustflags list for this invocation.
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# If we're standing in a workspace, this is a normal cargo invocation.
if [ -f "./Cargo.toml" ]; then
    exec cargo "$@"
fi

cmd="${1:-}"
shift || true

# The user's `c` alias resolves short forms (t, b, c, r) to this script
# with the short form as the first arg. Normalize them.
case "$cmd" in
    t) cmd=test ;;
    b) cmd=build ;;
    c) cmd=check ;;
    r) cmd=run ;;
esac

workspaces=(zapgui zapcli zap_wire)

run_in_all() {
    local sub="$1"
    shift
    local status=0
    for ws in "${workspaces[@]}"; do
        echo
        echo "=== cargo $sub in $ws ==="
        ( cd "$ROOT/$ws" && cargo "$sub" "$@" ) || status=$?
    done
    return $status
}

case "$cmd" in
    test|check|build)
        run_in_all "$cmd" "$@"
        ;;
    run)
        echo "local_cargo.sh: 'run' is per-workspace. cd into the one you want." >&2
        exit 1
        ;;
    "")
        echo "local_cargo.sh: no cargo subcommand given" >&2
        exit 1
        ;;
    *)
        echo "local_cargo.sh at $ROOT: no Cargo.toml here." >&2
        echo "cd into zapgui, zapcli, or zap_wire, or use 't', 'c', or 'b'." >&2
        exit 1
        ;;
esac