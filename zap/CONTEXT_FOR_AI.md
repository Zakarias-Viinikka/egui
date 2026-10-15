# Context for a fresh AI

You've been dropped into the zap workspace. Read in this order. Don't read
the whole repo, most of it is noise.

## 1. The top-level shape

Read `README.md` first. It has the architecture, the wire protocol, the
egui patch, and the parts that will confuse you if you don't read it.

Then `CHAIN_CHEATSHEET.md` for the chain grammar. The escape rule, the
absolute-path rule and the `cd`/`cargo`/`run` steps are all there.

Then these, for specific subsystems:
`CHAIN_REVIEW_UI.md` (the review window's layout, buttons, flags),
`FILE_TRUST.md` (`.sh` / `.toml` approval),
`RETRY.md` (the retry record and verb).

## 2. The shared parser

`zap_wire/src/chain.rs` holds `parse`, `ParseError`, `Step` and `Verb`.
Both zapcli and zapgui call it. zapcli calls it first, so a bad path or an
unknown verb never reaches the socket. zapgui calls the same parse again.

`zapgui/app_core/src/chain/parser.rs` and `chain/step.rs` are one-line
re-exports. If you see `crate::chain::Step`, that's the same type.

## 3. What zapgui is doing

`zapgui/app_core/src/chain/` — step.rs (re-export), parser.rs
(re-export), play.rs (play forward in memory and run move detection),
writer.rs (the one routine that writes), report.rs (clipboard / popup
sentences), whitelist.rs (list file, matching), popup.rs (fading_popup
wrapper).

`zapgui/app_core/src/terminal_router/mod.rs` — socket state and the shared
pending-chain slot. Small.

`zapgui/app_core/src/terminal_router/listener.rs` — reads the whole stream,
dispatches on FrameKind (Request, CargoRun, RunFile, ChainRun), writes
replies.

`zapgui/app_core/src/terminal_router/route_requests.rs` — parses the
request shape, dispatches to chain / overwrite / last / retry /
focus-if-busy, auto-applies whitelisted chains (never when the chain has
zaprm, zapmv, rmdir, cargo or run, and never for an unapproved `.sh` or
`.toml`), and contains `handle_chain_run`, the blocking handler for a
chain that ends in a run step.

`zapgui/app_core/src/terminal_router/retry.rs` — the last finished
review's record and `build_retry_chain`. Written by the UI when a chain
finishes; read by `handle_retry` in route_requests. See RETRY.md.

`zapgui/app_core/src/terminal_router/file_trust.rs` — the `.sh` / `.toml`
approval store, keyed by (path, content hash). See FILE_TRUST.md.

`zapgui/app_core/src/terminal_router/stream_run.rs` — the shared process
loop both cargo and run use. `cargo_run.rs` and `run_file.rs` are the two
verbs' server sides.

`zapgui/app_core/src/terminal_router/surface_review.rs` — the surface
review window's data. Each file keeps its old and new text for the
full-file toggle.

`zapgui/app_core/src/last_change.rs` — the in-memory record behind `last`.

`zapgui/app_core/src/run_trust.rs` — run-target trust entries.

`zapgui/app_core/src/clipboard.rs` — xclip wrapper. Failed writes retain
their exact payload and record kind in memory. They show the popup
`Failed to copy error to clipboard.` The zapcli `error` command asks zapgui
to retry copying the retained payload; it remains pending until the retry
succeeds. Callers must not show another popup after a failed clipboard write,
or they can hide the clipboard-failure popup.

`zapgui/app/src/egui_main.rs` — startup, icon, listener spawn, ctx storage,
single-instance handoff.

`zapgui/app/src/egui_setup/drawing_requirements.rs` — the review window,
the surface review, the menu.

## 4. The diff engine and the viewer

`zapgui/app_core/src/text_diff/` — `rows.rs` (types), `pair.rs` (line
pairing with edit-vs-replace), `post_process.rs` (rows → render blocks),
`similarity.rs` (token similarity shared by pair.rs and moves.rs),
`moves.rs` (move detection across the whole chain).

`zapgui/every_page_ever/src/ReusableComponents/diff_viewer/ui.rs` — the
three-pass row renderer. Word wrap, per-side bar colours, move strips,
selectable text, wrap markers. If you touch the viewer, read this file
top to bottom once.

## 5. What zapcli is doing

The zapcli `error` command asks zapgui to retry copying the most recent
failed clipboard payload. If retry fails, the payload stays in zapgui
memory until a successful retry or zapgui exits.

`zapcli/app_core/src/command_router/mod.rs` — dispatch. Look at this to know
what verbs exist. `create`, `replace`, `overwrite`, `append`, `rm`, `mv`,
`mkdir` forward their block as a chain; `core_commands/chain.rs` is the
shared forwarder, and detects a chain that ends in a run step so it can
open a ChainRun session instead of a plain request. `head`, `tail`, `wc`,
`last` return peek lines. `cargo` and `run` open streaming sessions.

`zapcli/app_core/src/command_router/core_commands/mod.rs` — the list of
command files.

`zapcli/app_core/src/general_util/path_parsing.rs` — how `~`, `~/...`,
absolute, and relative paths get resolved on the client side.

`zapcli/app_core/src/general_util/socket.rs` — how zapcli talks to zapgui.
`open` for plain requests, `open_cargo`, `open_run`, `open_chain_run` for
streaming.

`zapcli/app_core/src/shortcuts/mod.rs` and its children — Ctrl+C and
Ctrl+Shift+C handling.

`zapcli/app/src/egui_setup/drawing_requirements.rs` — the terminal UI, the
`raw_input_hook`, the event loop.

## 6. zap_wire and the run flow

`zap_wire/src/lib.rs` — framed messages, all FrameKinds.
`zap_wire/src/argv.rs` — the command-line splitter.
`zap_wire/src/cargo_gate.rs` — the cargo allow-list.
`zap_wire/src/chain.rs` — the shared chain parser.

`zapgui/app_core/src/surface.rs` and `trust.rs` — the execution surface and
its stored record.

`zapgui/app_core/src/runner.rs` — the process runner (own process group,
merged streams, cap, kill on cancel).

## 7. The egui patch

`egui-src/crates/eframe/src/native/epi_integration.rs` — one function is
patched. Everything else in `egui-src` is stock 0.36.2. Don't touch anything
else in there unless you know why. It's the only place a hidden window
stops flashing at startup.

## 8. Don't

Don't `cargo update`. Don't change the egui version. The patch depends on the
exact source.

Don't add heuristic line-coloring to zapcli's UI. Use `LineStyle`.

Don't add new shortcut keys without reading the Ctrl+C section in `README.md`
— egui-winit steals some combinations before the app ever sees them.

Don't assume zapcli can reach a real shell. It can't. That's the point.

Don't change `DiffRow`, `RowKind` or `Side`. Word wrap, per-side colours and
the move strips are draw-time changes. Edit-vs-replace and move detection
change `pair.rs` and add modules, but they keep the row types.

Don't add a new place that reads the socket in the run-step path. The
ChainRun handler starts one watcher thread; two readers on one socket
steal each other's frames.

## 9. `similar::TextDiffable` isn't public

`pair_lines` and `pair_lines_full` inline the pairing body instead of
generic-over-diff. If you refactor, keep that in mind.