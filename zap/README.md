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

See CHAIN_CHEATSHEET.md. The parser is zap_wire::chain, with
`ParseConfig` carrying the hidden set and the extension allowlist.
`parse(input)` loads the real config; `parse_with(input, &config)`
takes one. zapcli parses first so a bad path or unknown verb never
reaches the socket; zapgui parses again. The chain tests use
`parse_with` with an empty config so they never read the user's files.
zapgui/app_core/src/chain/{parser,step}.rs are re-exports.

## Confirm and deny

The pending chain lives in one slot in terminal_router as an Arc<Chain>,
holding steps, diffs and per-file snapshots. The UI reads the slot each
frame.

Four buttons in a fixed row: Verify, Mark, a 168-point approve slot, and
Deny All. On an ordinary step the slot holds the green Confirm check; on
a cargo, run or chmod step it holds a wide two-line ALLOW CARGO / ALLOW
RUN / ALLOW EXEC button, accent-bordered in the verb's colour. Confirm
writes every unmarked step. Marked steps are skipped and kept for retry.
Deny All writes nothing and makes the whole chain the retry payload,
keeping Marks.

`confirm_gate` in drawing_requirements.rs is a pure function used by
both Confirm and the wide Allow button. It stops on the first wanted
step that fails its gate: cargo, run and chmod need both `verified` and
`allowed`; zaprm, zapmv and rmdir need `visited`. Marked steps are not
gated. The wide Allow button sets `allowed` for the shown step, then
runs the same gate and write path as Confirm.

A chain can show only some of its steps. `review_indices` on Chain is
the set of step numbers that need eyes; empty means every step, which
is what play_forward and retry produce. `steps_that_need_eyes` in
play.rs computes it: a verb reason (always_reviewed), a whitelist reason
(the step's path or a move's destination is not matched by the
whitelist), or an approval reason (the path's final write is rejected
by the .sh / .toml approval check). A verb reason marks that step only;
a path reason marks every step that touches that path. An empty result
auto-applies as before; a full list falls back to showing every step.
The blocks column, the caption's starting step and the count use the
visible list, but Mark, Verify, Verify-done, the `visited` flag, the
click guards and the failure index stay keyed by real step number. A
failure on a hidden step sets `revealed_all`, jumps to that step, and
keeps the window open.

A small pen button sits just left of the full-file pill when the shown
step's verb has_editable_target (create, mkdir, zapmv, rename).
Clicking it opens a single-line field over the first caption line,
filled with the resolved full path (or the bare new name for a rename).
Enter commits: `replan_with_target` in play.rs clones every applied
step, sets the edited one's target with Step::set_target, remaps every
later step whose path, dest, run-file or (for a mkdir) cwd sits under
the old target to the new one, and plays the chain forward. On success
the new Chain replaces the old one in the pending slot through
`replace_pending_if`, and the old chain's waiter receives
`ChainOutcome::Superseded`. Visited and verified are cleared for the
edited step and every remapped step. Escape, a second click on the pen,
loss of focus without Enter, a step change, a new chain or hiding the
window cancels. While the field is open, confirm, mark, verify and deny
ignore clicks and the error line reads "Enter applies the new name, Esc
cancels". The `overrides` list on Chain records the from, the to and
the number of followers; `report::build` adds one clause per override
whose step was applied.

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

## Whitelist, hidden paths and extensions

Three files in ~/.config/zap (honours XDG_CONFIG_HOME), all editable
from the whitelist menu (`whitelist` in zapcli):

- whitelist.txt - one absolute path per line. A regular file matches
  only that file; a folder (or a path that does not exist yet) matches
  by whole components. All-inside chains auto-apply; any file outside
  or any zaprm / zapmv / chmod / cargo / run / unapproved .sh / .toml
  goes to review. See FILE_TRUST.md for the .sh / .toml rule.
- hidden.txt - paths and bare names hidden from the AI. See "Hidden
  paths".
- extensions.txt - optional extension allowlist for create, replace,
  overwrite and append. Empty or missing allows every extension. See
  "Allowed extensions".

The menu has three sections, in order: Whitelisted folders, Hidden
files, Allowed extensions. Each is a list with a remove button per
entry and an add field. Entries are saved the moment they are added or
removed.

## Hidden paths

A hidden.txt line starting with / or ~/ hides that path and everything
below it. A bare name with no slash hides every path with a component of
that name at any depth. Matching is by whole path components, never by
string prefix. Hidden always wins over the whitelist. The chain parser,
every read built-in (cat, head, tail, wc, ls, cd), run, cargo and last
refuse a hidden path with the line "deliberately hidden by the user:
<path>".

## Allowed extensions

When extensions.txt has entries, only those extensions may be written by
create, replace, overwrite and append. zaprm, zapmv, mkdir, rmdir,
chmod, run and cargo are not checked. Extensionless paths (Makefile,
.gitignore) are refused. A token is 1 to 16 letters, digits, _, - or +;
a leading dot is stripped and the rest lowercased. A broken file (has
tokens, none valid) fails closed and refuses every checked verb. The
parser reads the file fresh on every parse, so a change applies to the
next chain.

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
+ pair_runs), post_process.rs (rows -> blocks), similarity.rs
(similarity, inline_spans, EDIT_THRESHOLD, closest_line, edit_distance,
tokenize), moves.rs, move_display.rs.

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