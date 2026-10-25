# Context for a fresh AI

Last updated: 2026-10-25

Read in this order; don't read the whole repo, most of it is noise.

## 1. Top-level shape

README.md first - architecture, wire protocol, egui patch.
Then CHAIN_CHEATSHEET.md for the grammar and escape rule.
Then CHAIN_REVIEW_UI.md, FILE_TRUST.md, RETRY.md as needed.

## 2. Shared parser

zap_wire/src/chain.rs holds parse, parse_with, ParseConfig, ParseError,
Step, Verb. `parse` loads the real config (hidden set and extension
allowlist); `parse_with` takes one. Both zapcli and zapgui
call the shared parser; zapcli first so bad paths never reach the
socket, zapgui again to enforce the same rules.
zapgui/app_core/src/chain/{parser,step}.rs are re-exports.
zap_wire/src/hidden.rs holds the HiddenSet matcher.

## 3. zapgui

zapgui/app_core/src/chain/ - play.rs (play forward + move detection +
steps_that_need_eyes + the edit-title pen's replan_with_target),
writer.rs (the one write routine, apply and apply_detailed), report.rs
(capped build, build_capped), whitelist.rs, popup.rs (notify_copied),
astgrep.rs (the ast-grep subprocess and its per-file output), plus
step / parser re-exports.

zapgui/fading_popup/ - the popup binary, its own cargo workspace. Not
touched by c t; build_both.sh builds it. Font copied in by
build_both.sh from ~/ProgStuff/egui/reusable_components/fading_popup.

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
streaming sessions). head / tail return PeekAndCopy (peek lines plus
the same text on the clipboard); wc / last return Peek. Unknown verbs
print `unknown command: <verb>`; blank Enter is Silent.

zapcli/app_core/src/general_util/instance_guard.rs - a second launch of
zapcli connects to /run/user/1000/zapcli.sock, sends "focus" and exits;
the running one raises itself. zapcli/app/src/egui_main.rs binds that
socket at startup.

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

The edit-title pen swaps the live chain without waking the waiter.
`replan_with_target` builds the new Chain; `replace_pending_if` in
terminal_router/mod.rs swaps it into the pending slot only when the
slot still holds the old Arc; the old chain's waiter gets
`ChainOutcome::Superseded(SupersededChain(new))` and the wait loop in
handle_chain_run reassigns `mine` to the new Arc and keeps waiting. Do
not use stash_pending_arc for a pen swap: it would make the waiter
answer "this chain was replaced" and quit.

DiffRow, RowKind and Side are shared by the diff engine, viewer and
clipboard formatters. DiffRow has `edit: bool` (true on a Change row at
or above EDIT_THRESHOLD; the viewer draws blue bars). Side has
`changed: Vec<Range<usize>>` (byte ranges from inline_spans that differ
from the paired line). Both are set only on Change rows. Use
`DiffRow::new` / `Side::new`; the constructors default the new fields
and every construction site already goes through them. Consumers to
keep in sync: text_diff::process (blocks), pair_lines_full,
debug_format (the clipboard diff), the viewer's layout and placement
(diff_viewer/ui.rs, especially `astgrep_pair_by_index` in play.rs which
builds rows directly), and the diff viewer's right-click text builders
(row_side_text, side_joined_text). A new field needs a decision in each
of those.

## 9. similar::TextDiffable isn't public

pair_lines and pair_lines_full inline the pairing body. If you refactor,
keep that in mind.