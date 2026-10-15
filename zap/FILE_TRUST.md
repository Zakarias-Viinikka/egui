# FILE_TRUST

`.sh` and `.toml` files are sensitive. Even inside a whitelisted
folder, writing one goes to review. Once the user confirms a write, the
file's content hash at that moment is stored, and a later chain that
touches the same path at the same state doesn't re-ask.

## The store

`~/.config/zap/file_trust/`, one JSON file per path, keyed by a hash of
the path (same pattern as `run_trust.rs`).

`FileEntry { path, sha, at_secs }`. `sha` is SHA-256 of the file's
content at approval time. `sha_of_current` returns the hash of the empty
input for a file that doesn't exist, so the record is meaningful for a
fresh create too.

## What triggers a check

`route_requests::handle_chain` gathers every step's path that isn't a
remove, move or rmdir, filters by `file_trust::is_sensitive`, and calls
`all_approved`. If anything in that list is not approved, the chain
goes to review even if every path is inside a whitelisted folder.

## What drops an approval

- A `zaprm` or `zapmv` step on the path, or an `rmdir` on the folder.
  The approval is removed before the write, so recreating at the same
  path re-asks.
- Any change to the file that makes its hash differ from the stored
  one. Nothing explicitly removes the entry; `is_approved` returns
  false when the hash doesn't match.

## What is not stored

`run_trust` (executing a file) and `file_trust` (editing a sensitive
file) are separate. A `.sh` file approved for editing is not implicitly
approved for running. A run entry approved for a specific (path, args)
does not skip the edit review, and vice versa.

## Where

`zapgui/app_core/src/terminal_router/file_trust.rs`.
