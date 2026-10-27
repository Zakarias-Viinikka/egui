# RETRY

The retry subsystem rebuilds a chain from the last review's unapplied
steps. One slot in `terminal_router/retry.rs`, one verb in zapcli, one
request handler in `route_requests.rs`.

## The record

`RetryRecord { steps: Vec<Step>, statuses: Vec<StepStatus> }`. One
global slot, `OnceLock<Mutex<Option<Arc<RetryRecord>>>>`, in
`zapgui/app_core/src/terminal_router/retry.rs`.

Written when:

- The UI confirms a chain, right after `writer::apply` returns.
- The UI clicks Deny All.
- The writer fails partway through a confirm (same call site, `Failed`
  statuses).
- `play_forward` fails before a chain exists: the parse succeeded but a
  step's file state was wrong. `handle_chain` and `handle_chain_run`
  both save a record with every step `Failed` before replying, so a
  retry can still be sent for a chain that never reached the window.

## Statuses

From `chain::report::StepStatus`:

- `Applied` — written. Not in the retry payload.
- `Marked` — user marked it for retry. In the payload.
- `Denied` — user said no. In the payload; a retry can attempt it.
- `Failed` — writer or play_forward failed on it. In the payload.
- `Pending` — never reached. Only used mid-flight.

## The verb

`retry` sends a `retry` request. `retry N` sends `retry N`.
`retry 2 5` sends `retry 2 5`. Handled by
`route_requests::handle_retry`.

Payload:

- `retry` with no args — every step whose status is not `Applied`.
- `retry N [M ...]` — exactly those steps (1-based, from the original
  chain), ignoring statuses.

`build_retry_chain` re-reads each file from disk via `play_forward`, so
the diff shown is against the file's current state. Nothing is applied
directly; the rebuilt chain goes into the same pending slot as any other
chain and shows the same review window.

## Dropping steps

`retry -N [M ...]` retries every unapplied step except the named ones.
Use it when a step that failed was already done in a previous chain, or
should be left out for any other reason. The reply says which steps were
dropped. The dropped steps aren't in the new chain at all — they aren't
reported as failed, denied, or marked.

`retry -N` doesn't take a replacement body: a body replaces a step you
are retrying, and a dropped step is not retried. Mixing included and
dropped step numbers in one line (`retry 2 -3`) is refused with a single
message.

## Limits

A retry of a run or cargo step is refused: resend it as a chain. Runs
are streams, not file writes, and can't be replayed from the record.

A retry with an empty payload replies `zapgui: nothing to retry`.
