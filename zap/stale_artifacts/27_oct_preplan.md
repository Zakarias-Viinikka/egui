# 27 Oct preplan

Decided 2026-10-27. A batch of independent fixes, not one feature. The
orchestrator preplan is a separate thing and stays out of scope.

## Decided

### Quoted paths with spaces

The built-ins `cat`, `head`, `tail`, `wc`, `ls` refuse a quoted path
with a space, so `cat "/home/x/a b.txt"` fails. The chain body verbs
`create`, `replace`, `overwrite`, `append` and a chain's `cd` line have
the same problem: a quoted path there becomes part of the path and the
absolute-path check refuses it. The outcome: all of those accept a path
wrapped in one matching pair of `"` or `'`. The exact strip rule is
undecided; the outcome is what matters.

### Extensionless files in the allowed-extensions list

`extensions.txt` can list dotted extensions like `rs` or `toml`. There
is no way to say "also allow files with no extension", so `Makefile`
and `.gitignore` are refused when the list has entries. The outcome:
clicking add in the Allowed extensions section with the field empty
adds an entry meaning "no extension allowed". The on-disk spelling is
undecided; a file without the entry still fails closed on extensionless
paths.

### Shell wrapper: zap p, -free to zap, kill-on-launch

Three changes to the `zap` shell wrapper installed by build_both.sh.

`zap p` copies the current directory and hands it to zapcli as
zapcli's new `cd` folder. The handoff uses the existing
instance_guard socket; no second socket is added.

`zap -free` becomes `zap`. The `-free` spelling goes away; there is no
alias.

Launching zapcli through `zap` kills any running zapcli and any
running zapgui first, then starts fresh. The fading popup and the
zapgui menu are not killed.

### Review window: right-side blocks column

The blocks column on the right of the review window is one block per
step. With a long chain the column runs off the screen with no way to
scroll. The outcome: it scrolls.

The blocks should also be drawn grouped by folder, so all blocks for
one folder sit together. Blocks keep their real step number and their
click behaviour; only the draw order changes. A step that touches
several folders groups with whichever folder it shares the most with;
the exact rule is undecided.

### Review window: no click guards

All click guards are removed: the 150 ms CLICK_GUARD on Confirm, Mark
and Deny All, and the 600 ms SPECIAL_CLICK_GUARD on the wide ALLOW
button. No zero-millisecond version is left behind.

### Review window: one-sided steps show one pane

A step with content on only one side shows only that side. A `create`
shows only the after pane; a `delete` (formerly `zaprm`) shows only
the before pane. A step with content on both sides keeps both panes.
The full-file pill and the right-click menu keep working on the single
remaining pane.

### Review window: banner names file types

When a chain only touches markdown and/or text files, the "changes
only affect x" banner also names the file type. A chain that touches
only `.md` says so; only `.txt` says so; both says so. Anything with
other extensions falls back to the existing wording.

### Retry can drop steps

The bare `retry` re-delivers every unapplied step; `retry N <body>`
replaces a body but cannot remove a step. The outcome: a way to
express "retry and drop these steps" so a chain that failed at step 17
can be retried with 16 of 17. Dropped steps are not replayed and the
report names them as dropped, not as unapplied. The bare and numbered
forms keep working unchanged. The exact grammar is undecided.

### Exec/run spawn failure

A `run` or `cargo` step in a chain can fail with `spawn cargo: No such
file or directory (os error 2)` even though the same command works
when typed in a terminal. The user believes it used to work. The
outcome: it works again. The hard constraint: typed text must still
never become shell execution. Only a `run` or `cargo` step the user
already approved in the review window may go through a shell, or
through whatever program-resolution step is chosen.

### zaprm to delete

The verb `zaprm` is renamed to `delete`. The old spelling is removed
entirely: word-to-verb match, CHAIN_CHEATSHEET.md, the drift guard,
tests, user-facing report sentences and tooltips all say `delete`. No
alias.

## Dropped

These are removed from the todo list. They are not this batch's
problem and not any future batch's either unless the user says
otherwise.

`s` and `cdf`. `<verb> --help` copies a description to the clipboard.
`edit` (fzf-pick a file, open in Mousepad). Wrap-arrow colour (already
fixed). Move-detection thresholds (works fine in practice). Copy
selection from the right-click menu. The marked-step preview showing
an earlier marked step's edit. The astgrep `.rs`-only note. The popup
multi-monitor bookmark.

## Out of scope

The orchestrator preplan and every orchestrator file. No change to
zapcli's human-facing behaviour beyond the items above. No change to
chain parsing beyond the quoted-path item. No web client, no Android
app.

## Files to read

This is a reference list, not an instruction. Read what each item
touches before deciding how to solve it.

/home/zakke/ProgStuff/egui/zap/README.md
/home/zakke/ProgStuff/egui/zap/CONTEXT_FOR_AI.md
/home/zakke/ProgStuff/egui/zap/NOTES_FOR_NEXT_AI.md
/home/zakke/ProgStuff/egui/zap/CHAIN_CHEATSHEET.md
/home/zakke/ProgStuff/egui/zap/build_both.sh
/home/zakke/ProgStuff/egui/zap/local_cargo.sh
/home/zakke/ProgStuff/egui/zap/zap_wire/src/chain.rs
/home/zakke/ProgStuff/egui/zap/zap_wire/src/extensions.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/general_util/path_parsing.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/general_util/instance_guard.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/command_router/mod.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/command_router/core_commands/cd.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/command_router/core_commands/cat.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/command_router/core_commands/ls.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app/src/egui_main.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app/src/egui_setup/drawing_requirements.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/chain/play.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/chain/whitelist.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/chain/mod.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/terminal_router/retry.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/terminal_router/run_file.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/terminal_router/cargo_run.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/runner.rs
/home/zakke/ProgStuff/egui/zap/zapgui/every_page_ever/src/ReusableComponents/diff_viewer/ui.rs
