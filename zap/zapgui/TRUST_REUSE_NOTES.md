# Trust reuse for run/cargo steps

## The problem

A chain that contains a `run` or `cargo` step always goes to chain
review, even when the target file (or the project surface, for cargo)
has not changed since the user last approved it. That means re-approving
the same script every time it is run.

## The fix (this change)

`handle_chain_run` checks, before stashing the chain for review, whether
the chain is a single run or cargo step whose target is already trusted
for this exact invocation. If so, the chain is applied and the run is
dispatched without opening the chain review window. The run handler
(`run_file::run` / `cargo_run::run`) then re-checks trust on its own and
runs without opening its own review either.

Anything else (multi-step chains, changed files, missing trust) falls
through to the existing review window unchanged.

Trust records live in `~/.config/zap/run_trust/` (per canonical file +
extra args) and `~/.config/zap/trust/` (per project root, cargo
surface). Both store a SHA of the approved content.

## Why this is not obviously safe

The fast path trusts a SHA. If the file changes after approval, the SHA
mismatches and we fall back to review. That much is fine.

What is NOT handled:

- Time-of-check to time-of-use. Between the SHA read and the actual
  execution, the file could be swapped. The window is short but real.
  (`run_file::run` does a second SHA read after its own review window
  closes, but the fast path never opened a review window, so that
  recheck does not happen here.)
- The file's *interpreter*. `run foo.sh` executes `/bin/sh foo.sh` (or
  whatever the shebang says). If the interpreter itself changes, we do
  not notice. Only the script bytes are hashed.
- For cargo: the surface collector walks `Cargo.toml`, `Cargo.lock`,
  `build.rs`, proc-macro `src/`, and `.cargo/config`. It does NOT
  enumerate what the compiler actually does. Build scripts can exec
  arbitrary commands; proc macros run arbitrary code at compile time.
  A clean surface match does not prove the build is side-effect-free,
  only that the inputs we look at have not changed.
- The surface collector is deliberately conservative ("when unsure,
  include more") but not exhaustive. New Rust features or unusual
  manifest layouts could slip a file past it.
- Env vars, `RUSTFLAGS`, and other out-of-tree inputs are not part of
  either hash.

## Bottom line

The fast path removes repeat prompts for unchanged targets. It is a
usability win, not a security boundary. Anything that was going to
review once will still review once. Only the *repeat* is skipped, and
only when the stored hash matches.

If in doubt, delete the records:

    rm ~/.config/zap/run_trust/*.json
    rm ~/.config/zap/trust/*.json

and the next run will be reviewed again.

If the assumption about which field holds the run target turns out to be
wrong, the fast path silently does not fire and everything falls back to
review. That is the failure mode we want.
