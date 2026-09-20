#!/usr/bin/env bash
CARGO_TERM_COLOR=always RUSTFLAGS="-C link-arg=-fuse-ld=mold" \
  cargo test --workspace "$@" 2>&1 \
  | stdbuf -oL awk ' 
    BEGIN { mode="normal"; buf=""; in_warn=0 }
    {
        orig = $0
        line = $0
        gsub(/\033\[[0-9;]*m/, "", line)
    }
    in_warn {
        if (line == "") in_warn=0
        next
    }
    line ~ /warning:/ { in_warn=1; next }
    line ~ /Running (unittests|tests\/)/ || line ~ /Doc-tests/ {
        mode="collecting"; buf=orig "\n"; next
    }
    mode == "collecting" {
        buf = buf orig "\n"
        if (line ~ /running 0 tests/) { mode="skipping"; buf=""; next }
        if (line ~ /running [0-9]+ test/) {
            printf "%s", buf; fflush()
            mode="printing"; buf=""; next
        }
        next
    }
    mode == "skipping" { next }
    mode == "printing" { print orig; fflush(); next }
    { print orig; fflush() }
  '
