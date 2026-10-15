# zap

Two binaries, one workspace.

`zapgui` is a diff viewer. Hidden at startup. Appears when zapcli tells it to
show something.

`zapcli` is a fake terminal. You launch it by typing `zap` in a real terminal.
It looks like a terminal but only knows a small set of safe commands, plus a
`zap` verb that forwards raw text to zapgui.

They talk over a Unix socket at `/run/user/1000/zap.sock`.

## Why zapcli instead of a real terminal

The AI writes text. The text goes into zapcli. zapcli parses it (or forwards
it). zapcli never passes anything to a real shell. A mangled heredoc or stray
backtick can't execute anything, because zapcli is the one reading the input,
not bash.

## Why zapgui starts hidden

It would flash on screen at startup otherwise, and the user hates that. To
keep it hidden, eframe was patched. See "The egui patch" below.

## Layout

    zap/
      zapgui/       the diff viewer
      zapcli/       the fake terminal
      zap_wire/     the framed transport, the shared chain parser, the
                    cargo allow-list
      egui-src/     frozen + patched egui 0.36.2
      local_cargo.sh  from the root, `c t` tests all three workspaces
      build_both.sh builds and installs both
      README.md
      CONTEXT_FOR_AI.md

`zapgui/` and `zapcli/` are their own cargo workspaces. `zap_wire` is a
third sibling workspace that both depend on. That way both can have their
own `app`, `app_core`, `every_page_ever` without colliding.

## The egui patch

`egui-src/crates/eframe/src/native/epi_integration.rs` — the `post_rendering`
function's call to `window.set_visible(true)` on the first frame has been
removed. Upstream eframe force-shows the window after the first frame (egui PR
2279), which defeats any attempt to start hidden. The patch removes that, and
the app is responsible for calling `ViewportCommand::Visible(true)` when it
wants to appear.

Wired via `[patch.crates-io]` in `zapgui/Cargo.toml`. `egui-src` is committed
to the repo on purpose so the patch travels with it.

## Wire protocol

Request goes zapcli → zapgui. Reply goes zapgui → zapcli. Same socket, full
duplex. Frames are `[kind: u8][length: u32 LE][bytes]`, defined in
`zap_wire::lib`.

Request shapes:

`Request` with text `chain <cwd>\n<chain block>` — a chain that only does
file work. zapgui parses the block, plays it forward in memory, then either
applies it (if every touched file is inside a whitelisted folder and the
chain has no `zaprm`, `zapmv`, `cargo` or `run`) or stashes it for review and
raises the window. Reply is empty unless a failure or a displaced chain
happened; in those cases the reply is a one-line report and the same
sentence lands on the clipboard with a popup.

`Request` with text `overwrite <abs path>\n<content>` — kept for the old
one-step bare-overwrite shape; internally becomes a one-step chain. This
path is only reachable if something bypasses zapcli's parser (e.g. an
older client). zapcli itself routes every `overwrite` through the chain
parser.

`Request` with text `focus` or `focus <anything>` — zapgui brings itself
to the front, replies empty. (Was called `replace`; renamed to free the
word for the replace verb.)

`Request` with text `menu` — zapgui shows its general menu, replies empty.

`Request` with text `last [<path>]` — a peek-only reply from zapgui's
in-memory last-change record.

`Request` with text `retry` or `retry <n> [<n>...]` — rebuild a chain
from the last finished review's unapplied steps and stash it for review.
Replies empty unless there's nothing to retry or the request is
malformed. See RETRY.md.

`Request` with text `focus-if-busy` — reply is `shown` if a chain or
surface review is waiting (and zapgui is asked to show itself), `idle`
otherwise. Used by the external zap-front shortcut so it can hand off to
zapgui instead of focusing zapcli.

`CargoRun` frame, text `<cwd>\0<arg1>\0...` — the standalone `cargo`
verb. Streams output into zapcli's bottom pane.

`RunFile` frame, same text shape — the standalone `run` verb. Same stream.

`ChainRun` frame, text `<cwd>\n<chain block>` — a chain that ends in a
run step. The connection stays open through review, the writes, and the
run. Nothing streams until the user confirms.

`Cancel` frame, text empty — ask the current run to stop.

Reply shapes from zapgui to zapcli:

`Reply` — a single-line answer to a plain request. Followed by EOF on the
socket. zapcli reads until the connection closes.

`Notice` — a line for zapcli's main scrollback.

`StreamOpen` / `StreamData` / `StreamClear` / `StreamClose` / `StreamDone`
— the streaming frames the bottom pane consumes. One connection carries
one run.

Anything else replies `zapgui: got "<request>"`.

## The shared parser

`zap_wire::chain` holds `parse`, `ParseError`, `Step` and `Verb`. zapcli
calls `parse` first, so a bad path or an unknown verb never reaches the
socket. zapgui calls the same `parse` again and enforces the same rules.
The absolute-path requirement and the cargo allow-list live in the one
place.

`zapgui/app_core/src/chain/parser.rs` and `chain/step.rs` are one-line
re-exports, so old code that says `crate::chain::Step` still compiles.

## Chain grammar

See `CHAIN_CHEATSHEET.md` for the operator's view. Short version:
`create`, `replace`, `overwrite`, `append` carry content between `@new`
/ `@old` / `@end` markers. `zaprm`, `zapmv`, `mkdir`, `rmdir` are one-liners. A chain may
end with a `cd <folder>` line and a `cargo <args>` or `run <file> [args]`
line; that run step must be the last step and needs its `cd`.

Marker escape: `@@new` / `@@old` / `@@end` at the start of a content line
means one literal `@`. Only lines that collide with a marker word are
escapes. `@@ -1,3 +1,4 @@` and `@derive(Debug)` pass through untouched.

Chain writes require an absolute path or a `~/...` path. Reads do not.

## Run steps inside a chain

A chain that ends in a run step arrives as `ChainRun`, not `Request`.
The handler runs on the connection's own thread and blocks for the life
of the chain:

1. It parses the block, plays it forward, wraps the chain in an `Arc`,
   and stashes it. If a chain was already there, the displaced one's
   `notify` gets `ChainOutcome::Replaced` and a popup.
2. It starts one watcher thread on a clone of the stream. Cancel or EOF
   sets an `Arc<AtomicBool>`.
3. It waits on the chain's `mpsc` channel, polling the cancel flag and a
   600-second deadline.
4. When the UI confirms, the sender in `Chain.notify` sends
   `ChainOutcome::Applied(wanted)`. The handler looks up the run step. If
   the run was wanted, it calls `cargo_run::run` or `run_file::run` with
   the same cancel flag, so the run uses the watcher's flag instead of
   spawning a second reader on the same socket. Output streams through
   the standard frames. When the run finishes, the summary appends under
   the chain's report, already on the clipboard as `RecordKind::Report`.
5. On `AllDenied` the handler replies a one-line sentence. On `Replaced`
   the same. On cancel or timeout it calls `clear_pending` and replies.

The UI's `clear_pending` on a successful confirm happens before the
handler sees the outcome; the handler's own `Arc` keeps the chain alive
so the run step's path and args are still available.

## Confirm and deny

The pending chain lives in one slot in `terminal_router`, as an `Arc<Chain>`
holding the steps, their diffs, and a snapshot of each touched file as it
was when the chain was built. The UI reads the slot every frame and renders
from the last Arc it saw.

Four glass buttons below the diff. Left to right: Verify, Mark,
Allow/Confirm, Deny All.

Confirm writes every step that isn't marked. A step marked for retry is
skipped and stays in the retry payload (see RETRY.md). Verify is a
per-step flag: it's what lets Confirm write a run or cargo step; without
Verify on a run step, Confirm refuses and jumps to that step with a
message. Mark toggles the shown step's "leave for retry" state and turns
its block amber. Deny All writes nothing, puts the whole chain into the
retry payload, and preserves any Marks set before it.

If a step fails at write time (its file changed on disk since the
snapshot, a create target already exists, a replace old text no longer
matches), nothing is written and the window stays with a red error line,
and the failure report goes on the clipboard with a popup. A failure
report names the step index and verb:
`zap: nothing was written. step 3 of 7 (replace /abs/x.rs) failed: ...`.

A chain with more than one step shows a column of glass blocks on the
right, one per step. Each block's border is tinted by the step's verb
(create/edit/delete/rename/cargo/run each have a distinct accent). The
shown step's block is brighter; marked steps are amber; the caption
above the diff carries a matching accent bar on its left and reads
`step n of m · verb path`, with ` · marked for retry` or
` · verified` appended when the shown step carries that flag.

Below the caption, a dimmer line names the deepest common folder of
every path the chain touches: `edits confined to /abs/folder/`. It is
computed once in `play_forward`, over all step paths and move
destinations, and stored on the Chain as `boundary`.

A chain with more than one step shows a column of glass blocks on the right,
one per step. The shown step's block is brighter, denied blocks are red,
and clicking a block shows that step. The caption above the diff says
`step n of m · verb path`, and appends ` · denied` when the shown step has
been denied.

On success the slot empties (only if it still holds the same Arc, so a newer
chain survives) and the window hides with `ViewportCommand::Visible(false)`,
not `Close`, so the process and its listener stay alive.

Clicks are ignored for 500 ms after the displayed diff changes, after a
Mark, or after the window appears, so a click meant for something else
can't land on a freshly shown diff.

A new chain while one is showing replaces it. The reply and clipboard both
say `zap: replaced unconfirmed chain (<first path>).` so it doesn't
disappear silently.

## The viewer

The diff viewer is `every_page_ever::ReusableComponents::diff_viewer::ui`.
It draws rows in three passes.

Pass 0, before the scroll area: each side's text is laid out as a galley
from a `LayoutJob` in monospace. The wrap width is the side's inner width
minus the gutter, the bar, the text pad and a scrollbar reserve.
`break_anywhere` is on so long tokens wrap. Row height is
`max(ROW_MIN_H, taller galley + pad)`, plus a `TAG_H` strip above the
row if a move span starts on that line. A `LaidRow` records the galleys,
line numbers, move-span membership, tag labels, and the computed height.

Pass 1, inside the scroll area: each row is allocated at its computed
height and drawn. Bars span the row's height below any tag strip. Line
numbers pin to the first wrapped line. A move-span tag, if any, is painted
as a violet strip above the row's first wrapped line.

Pass 2: the text. Each side's pre-laid galley is placed as a
`Label::new(galley).selectable(true).wrap_mode(Wrap)` in a small child UI
with a top-left layout. Placing the labels in the order rows are drawn
works because egui's `LabelSelectionState` constrains a drag to labels
whose x-range overlaps the current selection. Left and right columns have
disjoint x-ranges, so a drag stays in one column.

Bar colours: left red for a `Delete`, right green for an `Insert`, left
red and right green for a `Change`, violet on both ends of a move span.
Nothing is coloured by inspecting row text.

Row 0 of a wrapped line has no marker. Each continuation row gets a
bright orange arrow with a short accent bar to its left, in a widened gap
between the bar and the text.

A small glass pill labelled "full file" / "changes only" sits at the top
right of the viewer area, outside the scrolling content. It swaps the
viewer between the 3-line-context snippet and every line of the file,
re-paired with `pair_lines_full`. The scroll id includes the step index
and the mode, so scroll position isn't shared.

## Reports and popups

Every decision (a step denied, Confirm, a failure, a whitelist auto-apply)
rebuilds a report: a plain sentence, one clause per file, saying what was
applied or denied or failed, with a bit of the diff as context. The report
goes on the clipboard, and a popup with a short label goes on screen.

Popups are the `fading_popup` binary in
`~/ProgStuff/egui/reusable_components`. `build_both.sh` builds it with the
`RUSTFLAGS` it already exports and installs it to `~/.local/bin`. A popup
appears top-center-ish, sits for 800 ms, fades, exits. Click to dismiss.
They are drawn directly against X11 (`override_redirect`) because an egui
window on XFCE fights the window manager for focus.

## Clipboard recovery

A failed clipboard write retains the exact payload in zapgui memory and
shows the popup `Failed to copy error to clipboard.` Run `error` in zapcli
to retry copying that retained text. The command reports the result in
zapcli. If the retry fails, the payload remains pending; it is cleared only
after a successful retry. The pending payload is lost when zapgui exits.

The failure popup is shown after the operation's ordinary popup would have
appeared, so a clipboard failure is not hidden by a success or edit-failure
message. The clipboard cannot be relied on to carry its own failure report.

## Whitelist

`~/.config/zap/whitelist.txt` (honours `XDG_CONFIG_HOME`), one absolute
folder per line. A chain whose touched files are all inside a whitelisted
folder is applied immediately — no diff, no window, just a
`Change was made` popup and the report on the clipboard. A chain with even
one file outside goes wholly to review.

A file counts as inside a folder after resolving symlinks and `..`, compared
by whole path components, not string prefixes. For a file that doesn't exist
yet, its nearest existing parent is used.

A chain with any `zaprm`, `zapmv`, `cargo` or `run` step goes to review even
inside a whitelisted folder, so an auto-applied edit can never be paired
with an auto-applied delete, rename or execution.

A `.sh` or `.toml` file whose content has not been approved at that exact
state also goes to review, even inside a whitelisted folder. Approval is
per (path, content hash) and sticks until the file changes. See
FILE_TRUST.md.

When zapgui is launched normally (without `--hidden`) it shows a menu. One
entry for now: Whitelisted folders. zapcli's auto-launch passes `--hidden`
so a zapgui woken by a request stays hidden. A manual launch while another
zapgui is running connects to the socket, asks that one to show its menu,
and exits — so a second launch can't steal the first one's socket.

## Cargo and streaming

zapcli's `cargo` verb runs cargo in zapgui and streams the output back into
a pane at the bottom of the zapcli window. zapcli itself never starts a
process — every side effect stays behind zapgui, which holds the guardrail
and the trust records. See `cargo` in `CHAIN_CHEATSHEET.md` for the
allow-list.

Transport is framed: `[kind: u8][length: u32 LE][bytes]`. zapcli keeps its
write half open so a cancel frame can go out while output is flowing. One
connection carries one request. A plain request is request, reply, close. A
run is CargoRun or RunFile, then StreamOpen, StreamData..., StreamDone. A
chain-with-run is ChainRun, then the same stream.

Before every run, zapgui computes the project's execution surface — every
Cargo.toml, Cargo.lock, build script, proc-macro source, .cargo/config, and
rust-toolchain file that would run or change what runs during a build. If
the surface matches the trusted record for that project, the run starts
immediately. Otherwise a surface review window shows the new or changed
files and asks for a decision. Confirm stores the surface and starts;
Deny cancels the run.

Ordinary source files and `target/` are not in the surface, so editing the
program he is deliberately running never re-prompts. After a run, only
Cargo.lock's trusted copy is refreshed. When a chain is confirmed in review,
any surface file it wrote in an already-trusted project gets its trusted
copy updated, because that review was the approval. Whitelisted
auto-applies do not do that.

## Line styles

Every command returns a `Vec<Line>`. `Line` has `text`, `style`, and
`skip_copy`. `LineStyle` is Normal, Error, Folder, Prompt, Zap, CliMessage,
Peek. The UI maps style to colour. The UI never inspects line text to guess
what a line is — that's what the styles are for. If you add a command,
decide per line what it means.

Colours: Normal white, Error red, Folder yellow, Prompt dim green, Zap cyan,
CliMessage gray, Peek muted violet (with a thin bar down the left edge).

`skip_copy` is set by the peek commands (`head`, `tail`, `wc`, `last`). A
line with `skip_copy = true` is left out of `copy` but included in
`copy all`. Peek commands set it on their own prompt line and every output
line, so a paste of the scrollback to an AI doesn't include the file being
inspected.

## Reading files and the last change

`head`, `tail`, `wc` behave like their GNU counterparts (`-n N`, `-nN`,
`-N`, `-l -w -c`). Output is violet with a bar, and goes through the skip
rule above.

`last [path]` shows the last change zap made to a file, diffed against the
file as it is now. With no argument, the most recently changed file. Records
live in memory in zapgui (module `last_change`), one per path, capped at
200, dropped oldest-first. A file too big or binary is recorded as a size
only, and `last` reports that. Only changes made since zapgui started are
known.

## The run verb

`run <file> [args]` runs one file. It never runs a command found on PATH.
The file is resolved against zapcli's cwd, canonicalized, and must be a
regular file. If it has the executable bit it runs directly; if not but it
ends in `.sh` it runs under `bash`; anything else is refused.

Before running, zapgui reads the file and looks up a trust entry keyed by
(canonical path, exact args). If the file's SHA-256 matches the stored one,
it runs without a prompt. Otherwise the EXECUTE REQUEST window opens with
a diff from the old approved text, or the full file on first use. Confirm
stores the entry and runs. Deny cancels with a clipboard sentence and a
popup. The file is re-read after confirm; if it changed while the window
was up, it refuses.

The environment matches `cargo` (colour off, progress bar on, `TERM=xterm`).
A script that calls cargo shows cargo's bar in the pane. `stream_run.rs` is
the shared loop both verbs use.

A run step inside a chain does the same trust check. A run target the chain
itself wrote is already trusted because the write was reviewed as a diff.

## Rename-aware trust

A trust entry means: this exact content, at this path, with these args, was
approved. A rename through a confirmed `zapmv` step re-keys every entry whose
path was the source, keeping the same sha and text. At the next `run`, if
the content is unchanged, no prompt. If the content changed since approval,
the review shows a diff from the old approved text.

A `zaprm` step drops the entries for that path. Recreating a file at the same
path is new and needs a full review. A chain with any `zaprm` or `zapmv` skips
the surface content refresh — moved content never appeared as a diff, so
nothing is auto-trusted. Chains without `rm` or `mv` keep the old behaviour
of refreshing the cargo surface for files the chain wrote.

The RENAME REQUEST caption adds a `· trusted for running` note when the
source file is a run target or a file in a trusted cargo surface.

## Edit-vs-replace

`zapgui/app_core/src/text_diff/pair.rs`. When `similar` reports a run of
deleted and inserted lines, the pairer aligns old and new by token-level
similarity instead of by index.

Two lines are similar if the weighted longest common subsequence of their
tokens — runs of letters, digits and underscore — covers at least half
their total token weight. `SIMILARITY_THRESHOLD` is 0.5. Above the
threshold, the pair is one row: `Equal` if identical, else `Change`. Below
the threshold, the old line and new line are not paired; the run's
unmatched deletes are emitted before the unmatched inserts, never
interleaved.

Above `ALIGNMENT_CELL_LIMIT` (100,000 old-times-new) the DP is skipped
and lines are paired by index with the same threshold test.

## Move detection

`zapgui/app_core/src/text_diff/moves.rs`. Detection runs once over every
step of a chain, not once per step, so a move between two files is caught.

Candidates are maximal runs of consecutive Delete-only rows and of
Insert-only rows. Runs smaller than 3 lines or 60 non-whitespace characters
are ignored. A Delete run and an Insert run that touch directly are never
paired; that's a replacement.

Cheap pre-check: Dice over the multisets of tokens must reach
`DICE_FLOOR` (0.6). Then confirm with the alignment function from
edit-vs-replace; matched weight over the longer run's weight must reach
`ALIGN_FLOOR` (0.7). If either run exceeds `LONG_RUN` (300 lines), skip
alignment and require `DICE_FLOOR_LONG` (0.8). Take pairs greedily by
descending score, each run used once. `edited` is true when the matched
lines aren't all identical after trimming.

Each confirmed pair produces a `MoveSpan` on the From step and one on the
To step. A span carries `role`, `first_line`, `last_line`, `label`,
`edited`. Labels are built here where paths are known, e.g.
`moved to src/foo.rs:120` or `moved from /abs/a.rs:5 (edited)`.

The viewer decides membership from line numbers only: a Delete row whose
left line number is inside a From span, an Insert row whose right line
number is inside a To span. A member row gets a violet bar on the owning
side. A span starting on a row paints a violet strip above that row with
the label.

## The Ctrl+C problem

egui-winit converts Ctrl+C to `Event::Copy` regardless of whether Shift is
held (`is_copy_command` only checks `modifiers.command`). That means Ctrl+C
and Ctrl+Shift+C are indistinguishable by the time events reach the app. To
work around this, zapcli's `raw_input_hook` strips both the Key(C) event and
any `Event::Copy` before egui processes them. It reads modifiers from the Key
event when available and falls back to ClearInput when only `Event::Copy` came
through. The `copy` command is the reliable way to copy everything.

Inside the zapgui viewer, Ctrl+C on selected text goes through egui's own
selection handling, which works because the labels are selectable.

## The input line

The input is an `egui::TextEdit` (multiline, one row tall, frame off). It
gives Ctrl+Z, Ctrl+Y, selection, Ctrl+A, cut and click-to-place cursor
without extra code. Plain Enter submits; Shift+Enter inserts a newline; Tab
is stripped in `raw_input_hook` so it can't move focus to a pane button.
Focus is requested at most once per frame and only when nothing else holds
it — requesting focus every frame spins the render loop.

## Where files go

`zapgui/app_core/src/chain/` — the chain module. `step.rs` and `parser.rs`
are re-exports of `zap_wire::chain`. `play.rs` plays forward in memory and
runs move detection. `writer.rs` is the one routine that writes.
`report.rs` is the clipboard / popup sentences. `whitelist.rs` is the list
file and matching. `popup.rs` wraps `fading_popup`.

`zapgui/app_core/src/terminal_router/` — socket listener,
`route_requests`, the blocking `handle_chain_run`, the shared slot in
`mod.rs`. The slot holds an `Arc<Chain>`.

`zapgui/app_core/src/terminal_router/retry.rs` — the last finished
review's record and the `retry` verb's builder. See RETRY.md.
`terminal_router/file_trust.rs` — the `.sh` / `.toml` approval store.
See FILE_TRUST.md.

`zapgui/app_core/src/terminal_router/stream_run.rs` — the shared process
loop both `cargo` and `run` use. `cargo_run.rs` and `run_file.rs` are the
two verbs' server sides.

`zapgui/app_core/src/terminal_router/surface_review.rs` — the surface
review window's data. Each `SurfaceReviewFile` keeps its `old_text` and
`new_text` so the full-file toggle can re-pair them.

`zapgui/app_core/src/clipboard.rs` — the xclip helper, returning an error
string when xclip can't start or exits non-zero.

`zapgui/every_page_ever/src/ReusableComponents/` — `close_button`,
`glass_button` (including `pill_ui`), `diff_viewer`. Each has a `mod.rs`
and `ui.rs`.

`zapcli/app_core/src/command_router/` — dispatch, plus
`core_commands/<name>.rs` per verb. `core_commands/chain.rs` forwards
create, replace, overwrite, append, zaprm, zapmv, mkdir and run chains; the
cargo and run verbs live inline in `mod.rs` because they open streaming
sessions.

`zapcli/app_core/src/general_util/socket.rs` — the framed client: `open`
for plain requests, `open_cargo`, `open_run`, `open_chain_run` for
streaming.

`zapcli/app_core/src/general_util/path_parsing.rs` — how `~`, `~/...`,
absolute and relative paths get resolved on the client side.

`zapcli/app/src/egui_setup/drawing_requirements.rs` — the terminal UI, the
stream pane, the `raw_input_hook` for Ctrl+C and Ctrl+Shift+C.

`zapgui/app/src/egui_setup/drawing_requirements.rs` — the review window,
the surface review, the menu.

`zapgui/app_core/src/text_diff/` — `rows.rs` (types), `pair.rs` (line
pairing), `post_process.rs` (rows → render blocks), `similarity.rs`
(token similarity shared with moves), `moves.rs` (move detection).

`zapgui/app_core/src/last_change.rs` — the in-memory record of the last
change per file.

`zapgui/app_core/src/run_trust.rs` — run-target trust entries, one per
(canonical path, exact args), under `~/.config/zap/run_trust/`.

`zap_wire/src/chain.rs` — the shared parser.

## Adding a command

Create `zapcli/app_core/src/command_router/core_commands/<name>.rs`. Write
`pub fn run(args: &[&str], session: &Session) -> Outcome`. Add it to
`core_commands/mod.rs`. Add a match arm in `command_router/mod.rs`. Return
`Vec<Line>` in `Outcome::Print` — pick a `LineStyle` per line.

If the command needs to forward a multi-line block to zapgui, take the
verbatim input (not `trimmed`) and send it as `chain <cwd>\n<block>`. See
`core_commands/chain.rs` for the existing create and replace verbs.

## Build

`~/ProgStuff/egui/zap/build_both.sh`. Builds and installs `zapgui`,
`zapcli`, `zap-settings`, the `zap` wrapper (`~/.local/bin/zap`, a bash
script that execs `zapcli`), and `fading_popup` from
`~/ProgStuff/egui/reusable_components`. It also kills a running zapgui and
starts a fresh hidden one, so a rebuild takes effect immediately.

`zap_wire/` is a small crate both binaries depend on: the framed-message
types, the shared chain parser, and the cargo allow-list. It lives beside
`zapgui/` and `zapcli/` as a sibling workspace. Any change in `zap_wire`
means both binaries must be rebuilt.

From the zap root, `c t` runs `cargo test` in zapgui, zapcli and zap_wire,
one after the other. `c c` for check, `c b` for build.

## Socket path

`/run/user/1000/zap.sock`. Hardcoded in two places:
`zapgui/app_core/src/terminal_router/mod.rs` as `SOCKET_PATH`, and
`zapcli/app_core/src/general_util/socket.rs` as `SOCKET`. If your user id is
not 1000, both need changing.