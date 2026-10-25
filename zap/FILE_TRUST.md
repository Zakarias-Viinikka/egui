# FILE_TRUST

`.sh` and `.toml` files are sensitive. Even inside a whitelisted
folder, a chain that writes one goes to review unless that exact write
was approved before.

## The key

The approval answers "this exact content may be written at this path".
An entry stores the path, the hash of the text a chain would leave in
the file, and a time. It is not a hash of whatever is on disk right now.

A chain can touch one file in several steps. The write that matters is
the last one: `Chain::final_writes(wanted)` returns the path and after
text of the last content-writing step per file, honoring the wanted
steps. Both sides of the approval use that method - the check before the
write passes an empty `wanted` slice (every step), and the record after
confirm passes the real one.

## The store

`~/.config/zap/file_trust/`, one JSON file per path, keyed by a hash of
the path (same pattern as `run_trust.rs`).

`FileEntry { path, sha, at_secs }`. `sha` is SHA-256 of the text the
chain writes at that path.

## What triggers a check

`route_requests::handle_chain` and `handle_overwrite` build the write
list with `final_writes`, filter to sensitive paths, and call
`all_approved`. If anything in that list is not approved, the chain
goes to review even if every path is inside a whitelisted folder.

`is_approved(path, write_text)` first applies the drift rule: if the
file on disk no longer hashes to the stored value, something other than
zap changed it after the approval, so the entry is deleted on the spot
and the answer is false. Otherwise it answers true only when the stored
hash equals the hash of the write. A file deleted out of band hashes as
empty input and revokes its entry the same way.

The consequence is intended: a changed `.sh` or `.toml` is always
reviewed, and an entry only short-circuits a write of exactly the
content it recorded.

## What records an approval

`refresh_trust_after_chain_confirm` calls `record_approved` with
`final_writes(wanted)` after a confirmed write. A step that was marked
and not written is not in that list, so it records nothing.

## What drops an approval

- A `zaprm` or `zapmv` step on the path, or an `rmdir` on the folder.
  The approval is removed before the write, so recreating at the same
  path re-asks.
- Any change to the file out of band that makes its hash differ from
  the stored one. `is_approved` deletes the entry the first time it
  sees the mismatch.

## What is not stored

`run_trust` (executing a file) and `file_trust` (editing a sensitive
file) are separate. A `.sh` file approved for editing is not implicitly
approved for running. A run entry approved for a specific (path, args)
does not skip the edit review, and vice versa.

## Where

`zapgui/app_core/src/terminal_router/file_trust.rs`.