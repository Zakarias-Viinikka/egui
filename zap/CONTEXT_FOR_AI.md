# Context for a fresh AI

Last updated: 2026-10-25

Read in this order; don't read the whole repo, most of it is noise.

## 1. Top-level shape

README.md first - architecture, wire protocol, egui patch.
Then CHAIN_CHEATSHEET.md for the grammar and escape rule.
Then CHAIN_REVIEW_UI.md, FILE_TRUST.md, RETRY.md as needed.

## 2. Shared parser

zap_wire/src/chain.rs holds parse, parse_with, ParseConfig, ParseError,
Step, Verb. `parse` loads the real config (hidden set, later the
extension allowlist); `parse_with` takes one. Both zapcli and zapgui
call the shared parser; zapcli first so bad paths never reach the
socket, zapgui again to enforce the same rules.
zapgui/app_core/src/chain/{parser,step}.rs are re-exports.
zap_wire/src/hidden.rs holds the HiddenSet matcher.

## 3. zapgui

zapgui/app_core/src/chain/ - play.rs (play forward + move detection),
writer.rs (the one write routine), report.rs, whitelist.rs, popup.rs,
plus step / parser re-exports.

zapgui/app_core/src/terminal_router/ - mod.rs (socket state + pending
slot), listener.rs, route_requests.rs (dispatch, whitelist auto-apply,
handle_chain_run), retry.rs, file_trust.rs, stream_run.rs,
surface_review.rs.

zapgui/app_core/src/text_diff/ - rows.rs (types + constructors), pair.rs
(line pairing + pair_runs), post_process.rs (rows -> blocks),
similarity.rs (similarity, inline_spans, EDIT_THRESHOLD, dice, lcs),
moves.rs, move_display.rs.

zapgui/every_page_ever/src/ReusableComponents/diff_viewer/syntax.rs -
syntax tables for Rust and TOML. Two-face syntax/theme sets; one table
per side of a step, built once and cached.

zapgui/app_core/src/last_change.rs, run_trust.rs, clipboard.rs.
zapgui/app/src/egui_main.rs - startup, single-instance handoff.
zapgui/app/src/egui_setup/drawing_requirements.rs - review window,
surface review, menu.

## 4. The diff engine and viewer

zapgui/every_page_ever/src/ReusableComponents/diff_viewer/ui.rs - the
three-pass row renderer. If you touch it, read the file top to bottom
once.

## 5. zapcli

zapcli/app_core/src/command_router/mod.rs - dispatch. Look here to know
what verbs exist. core_commands/<name>.rs per verb, core_commands/chain.rs
forwards chains. cargo and run live inline in mod.rs (they open
streaming sessions). head / tail / wc / last return peek lines.

zapcli/app_core/src/general_util/{path_parsing,socket}.rs.
zapcli/app_core/src/shortcuts/ - Ctrl+C and Ctrl+Shift+C.
zapcli/app/src/egui_setup/drawing_requirements.rs - terminal UI,
raw_input_hook.

The zapcli `error` command retries the most recent failed clipboard
payload.

## 6. zap_wire and the run flow

zap_wire/src/lib.rs (frames), argv.rs (splitter), cargo_gate.rs
(allow-list), chain.rs (parser), config.rs (config folder, line lists,
key=value settings).

zapgui/app_core/src/surface.rs, trust.rs, runner.rs - the execution
surface, its stored record, the process runner.

## 7. The egui patch

egui-src/crates/eframe/src/native/epi_integration.rs - one function
patched. Everything else in egui-src is stock 0.36.2. Only place a
hidden window stops flashing at startup.

## 8. Don't

Don't cargo update. Don't change the egui version - the patch depends on
the exact source.

Don't add heuristic line-colouring to zapcli's UI. Use LineStyle.

Don't add shortcut keys without reading the Ctrl+C section in README.

Don't skip the hidden check. Any verb whose path argument can be the
one the user cares about must call `config.hidden.refusal(path)` (or
`zap_wire::hidden::HiddenSet::check` on the zapcli and zapgui sides).
Hidden always wins over the whitelist.

Don't assume zapcli can reach a shell.

Don't add a second socket reader in the run-step path - the ChainRun
handler starts one watcher; two readers steal each other's frames.

DiffRow, RowKind and Side are shared by the diff engine, viewer and
clipboard formatters. DiffRow has `edit: bool` (true on a Change row at
or above EDIT_THRESHOLD; the viewer draws blue bars). Side has
`changed: Vec<Range<usize>>` (byte ranges from inline_spans that differ
from the paired line). Both are set only on Change rows. Use
`DiffRow::new` / `Side::new`; the constructors default the new fields
and every construction site already goes through them. A new field still
needs a decision about what the viewer, debug_format and the clipboard
builders do with it.

## 9. similar::TextDiffable isn't public

pair_lines and pair_lines_full inline the pairing body. If you refactor,
keep that in mind.