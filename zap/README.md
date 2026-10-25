# zap

Last updated: 2026-10-25

Two binaries, one workspace. zapgui is a diff viewer, hidden at startup.
zapcli is a fake terminal - parses or forwards, never executes. They talk
over a Unix socket at /run/user/1000/zap.sock. Shared crate zap_wire.

zapcli is not a shell on purpose: a mangled heredoc or stray backtick
cannot execute anything, because zapcli is the one reading the input.

zapgui starts hidden. eframe is patched at
egui-src/crates/eframe/src/native/epi_integration.rs to remove the
force-show on first frame. The app calls ViewportCommand::Visible(true)
itself when it wants to appear.

## Layout

zapgui/, zapcli/, zap_wire/ - each its own cargo workspace.
egui-src/ - frozen + patched egui 0.36.2, wired via [patch.crates-io].
local_cargo.sh - c t / c c / c b across all three.
build_both.sh - builds and installs zapgui, zapcli, zap-settings, the zap
                wrapper, fading_popup; restarts zapgui hidden.

## Wire protocol

Frames: [kind: u8][length: u32 LE][bytes]. Defined in zap_wire::lib.

Request shapes (zapcli -> zapgui):
- `chain <cwd>\n<chain block>` - file-only chain. Applied at once if
  every touched file is whitelisted and there's no zaprm / zapmv / cargo
  / run; else stashed for review. Empty reply on success.
- `overwrite <abs path>\n<content>` - legacy one-step chain. Only
  reached by clients that bypass zapcli's parser.
- `focus` - bring zapgui forward.
- `menu` - show the general menu.
- `last [<path>]` - peek at last_change.
- `retry [<n>...]` - rebuild the last review's unapplied steps. See
  RETRY.md.
- `focus-if-busy` - reply `shown` or `idle`. Used by the external
  zap-front shortcut.
- `CargoRun`, `RunFile`, `ChainRun` frames - streaming runs. Replies are
  StreamOpen / StreamData / StreamClear / StreamClose / StreamDone.
- `Cancel` - stop the current run.

Any other request replies `zapgui: got "<request>"`.

## Chain grammar

See CHAIN_CHEATSHEET.md. The parser is zap_wire::chain. zapcli parses
first so a bad path or unknown verb never reaches the socket; zapgui
parses again. zapgui/app_core/src/chain/{parser,step}.rs are re-exports.

## Confirm and deny

The pending chain lives in one slot in terminal_router as an Arc<Chain>,
holding steps, diffs and per-file snapshots. The UI reads the slot each
frame.

Four glass buttons: Verify, Mark, Allow/Confirm, Deny All. Confirm writes
every unmarked step. Marked steps are skipped and kept for retry. Verify
gates run and cargo steps. Deny All writes nothing and makes the whole
chain the retry payload, keeping Marks.

Write failures leave the window open with a red error line and put the
report on the clipboard with a popup. A chain with more than one step
shows a column of verb-tinted blocks on the right; clicking one jumps to
that step. The caption reads `step n of m - verb path`, with
` - marked for retry` or ` - verified` appended.

A boundary chip on the caption row, left of the full-file pill, names
the deepest common folder of everything the chain touches. Green
normally, amber at two path components or fewer, red-orange reading
`edits span the whole disk` for `/`. Shortens from the left with `...`
when it would exceed 45% of the caption area.

Clicks are ignored for 150 ms after the diff changes, a Mark, or the
window appears. Same guard on the surface review's Confirm and Deny All.

A new chain while one is showing replaces it, with a report and popup.

## The viewer

every_page_ever::ReusableComponents::diff_viewer::ui. Three passes.

Pass 0 lays out each side's galley at the row's computed height.
break_anywhere on. Row height = max(ROW_MIN_H, taller galley + pad) plus
TAG_H if a move span starts on that line.

Pass 1 allocates each row at its height and paints bars, line numbers
and any move strip. First row of any non-Equal block gets a thin rule
above it.

Pass 2 places the text as selectable Labels. Labels are placed per-pane
after the row loop (all left, then all right), so a drag stays inside
one pane.

Bar colours: red left for Delete, green right for Insert, red left and
green right for Change, violet on both ends of a move span. Row 0 of a
wrapped line has no marker; continuation rows get an arrow and accent
bar.

The full-file pill at the top right of the viewer swaps between the
snippet and every line of the file, re-paired with pair_lines_full.

## Edit-vs-replace

zapgui/app_core/src/text_diff/pair.rs. When similar reports a run of
deleted and inserted lines, the pairer aligns old and new by token
similarity instead of by index. Tokens are runs of letters, digits and
underscore; the weighted longest common subsequence must cover at least
SIMILARITY_THRESHOLD (0.5) of both lines' token weight. Above the
threshold the pair is one row - Equal if identical, else Change. Below,
the run's unmatched deletes are emitted before the unmatched inserts.

When a change block holds exactly one old line and exactly one new line,
they become one Change row regardless of similarity, so a short line
replaced by a longer one is one row.

Above ALIGNMENT_CELL_LIMIT (100,000 old-times-new) the DP is skipped and
lines are paired by index with the same threshold test.

Two thresholds decide how a paired Change row is drawn. At or above
EDIT_THRESHOLD (0.6) the row is an edit and both bars are blue. Between
SIMILARITY_THRESHOLD and EDIT_THRESHOLD it is a paired replacement, red on
the left and green on the right. Change rows carry `edit: bool` for the
bars and `changed: Vec<Range<usize>>` on each side for the spots.

The spots come from `inline_spans`. Each line is cut into tokens - runs
of letters, digits and underscore, runs of whitespace, single other
characters - and the two token lists are diffed with Myers. Every
delete/replace gives a range on the old side, insert/replace on the new
side. Replace ranges are shaved to their longest common prefix and
suffix at character level, so `compute_total` to `compute_totals` marks
only the `s`. Ranges merge when they touch. `inline_spans` returns
nothing when either line is over 2,000 characters, or when the marked
characters cover more than 70% of the non-space characters of either
line, so a mostly rewritten edit shows bars and no spots.

## Syntax coloring

`every_page_ever/src/ReusableComponents/diff_viewer/syntax.rs`. The
viewer draws sparse rows, so a line can't be coloured from its own text
without losing state: a line in the middle of a multi-line string would
look like code. Each side's whole text is highlighted top to bottom once
into a table indexed by line number; a row looks up its runs by the line
number it already carries.

syntect, with two-face's `extra_newlines` syntax set (Rust and TOML;
syntect's bundled set has no TOML) and two-face's themes. `rs` is Rust,
`toml` and `Cargo.lock` are TOML, anything else is an empty table and
draws as plain text. Texts over 400 KB, or with a single line over 5,000
bytes, also get an empty table. A failed build prints
`zap: <method> <file>: <message>` and draws plain.

Themes: `off`, `two-dark` (default), `dracula`, `nord`, `monokai`,
`solarized-dark`, `gruvbox-dark`, `one-half-dark`, `zenburn`. The
setting key is `theme` in the settings file. Set it in zap-settings; a
theme chosen there applies to the next step shown, with no restart.

In the viewer, pass 0 builds each row's LayoutJob from the pure function
`segments_for_line(len, runs, highlights) -> Vec<Segment>`. Each segment
is `{ start, end, color, italic }`; the segment carries no
"highlighted" flag, and the caller tests whether a segment lies inside a
range. Uncovered bytes come back as TEXT upright. Copy and selection are
unchanged because the segments feed the same galley the labels place.

## Move detection

zapgui/app_core/src/text_diff/moves.rs. Runs once over every step of a
chain, so a cross-file move is caught.

Candidates are maximal runs of consecutive Delete-only rows and
Insert-only rows, at least 3 lines and 60 non-whitespace chars. A Delete
run and Insert run that touch directly are a replacement, not a move.
Cheap pre-check: Dice over tokens must reach DICE_FLOOR (0.6). Then
align; matched weight over the longer run must reach ALIGN_FLOOR (0.7).
If either run exceeds LONG_RUN (300 lines), skip alignment and require
DICE_FLOOR_LONG (0.8). Pairs are taken greedily by descending score.

Each confirmed pair produces a MoveSpan on the From step and one on the
To step, carrying role, first_line, last_line, label, edited, pair (a
chain-unique id shared by the two spans), partner_step and
partner_first_line.

move_display::apply runs on a step's flat pair_lines rows before
process. For each To span whose partner is in the same step, it collects
the From span's old sides, pairs them with the To span's new sides via
pair_runs, and splices the result in as Change rows.

Display. Snippet view is Dimmed: From-side rows show as dim grey ghosts
with no bar or strip; To-side rows keep the violet bar and strip. Full-
file view and surface reviews are Classic (violet bar and strip on both
ends). The rule is set by MoveDisplay on DiffViewerParams.

## Reports and popups

Every decision (Confirm, deny, failure, whitelist auto-apply) rebuilds
a report sentence, puts it on the clipboard, and shows a fading_popup
label.

A failed clipboard write retains the payload in zapgui memory and shows
a failure popup. `error` in zapcli retries the copy. The payload is
lost if zapgui exits before a successful retry.

## Whitelist

~/.config/zap/whitelist.txt (honours XDG_CONFIG_HOME), one absolute
folder per line. All-inside chains auto-apply; any file outside or any
zaprm / zapmv / cargo / run / unapproved .sh / .toml goes to review. See
FILE_TRUST.md for the .sh / .toml rule.

## Line styles

Every command returns a Vec<Line>; each Line carries its own LineStyle
(Normal, Error, Folder, Prompt, Zap, CliMessage, Peek). The UI never
inspects text to guess a style. skip_copy marks peek output; copy skips
those, copy all keeps them.

## The Ctrl+C problem

egui-winit turns Ctrl+C into Event::Copy regardless of Shift. zapcli's
raw_input_hook strips both the Key(C) event and any Event::Copy before
egui sees them. Inside the viewer, Ctrl+C on a selection goes through
egui's own handling.

## Where files go

zapgui/app_core/src/chain/ - step / parser re-exports, play.rs (play
forward + move detection), writer.rs (the one write routine), report.rs,
whitelist.rs, popup.rs.

zapgui/app_core/src/terminal_router/ - socket listener, route_requests,
handle_chain_run, shared slot in mod.rs, retry.rs, file_trust.rs,
stream_run.rs, surface_review.rs.

zapgui/app_core/src/text_diff/ - rows.rs (types), pair.rs (line pairing
+ pair_runs), post_process.rs (rows -> blocks), similarity.rs, moves.rs,
move_display.rs.

zapgui/app_core/src/last_change.rs, run_trust.rs, clipboard.rs.

zapgui/every_page_ever/src/ReusableComponents/ - close_button,
glass_button (with pill_ui), diff_viewer (ui.rs the renderer, syntax.rs
the colour tables).

zapcli/app_core/src/command_router/ - dispatch + core_commands/<name>.rs
per verb. core_commands/chain.rs forwards chains. cargo and run live
inline in mod.rs.

zapcli/app_core/src/general_util/{socket,path_parsing}.rs.
zapcli/app/src/egui_setup/drawing_requirements.rs - terminal UI,
raw_input_hook.

zapgui/app/src/egui_setup/drawing_requirements.rs - review window,
surface review, menu.

zap_wire/src/chain.rs - shared parser.
zap_wire/src/config.rs - config folder, line-list files, key=value
settings. Settings writes go through a temp file and a rename so unknown
lines survive.

## Adding a command

Create zapcli/app_core/src/command_router/core_commands/<name>.rs. Write
`pub fn run(args: &[&str], session: &Session) -> Outcome`. Add it to
core_commands/mod.rs and a match arm in command_router/mod.rs. Return
Vec<Line> in Outcome::Print with a LineStyle per line.

## Build

~/ProgStuff/egui/zap/build_both.sh. From the zap root, c t / c c / c b
across all three workspaces.

## Socket path

/run/user/1000/zap.sock. Hardcoded in
zapgui/app_core/src/terminal_router/mod.rs (SOCKET_PATH) and
zapcli/app_core/src/general_util/socket.rs (SOCKET).