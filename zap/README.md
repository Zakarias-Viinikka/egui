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
      egui-src/     frozen + patched egui 0.36.2
      build_both.sh builds and installs both
      README.md
      CONTEXT_FOR_AI.md

Both `zapgui/` and `zapcli/` are their own cargo workspaces. They're siblings,
not one merged workspace. That way both can have their own `app`, `app_core`,
`every_page_ever` without colliding.

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
duplex.

Four request shapes:

`chain <cwd>` followed by a newline, then the chain block (see
`CHAIN_CHEATSHEET.md`). zapcli sends its working directory on the first line
so zapgui can resolve relative paths inside the block. zapgui parses the
block, plays it forward in memory, and then either applies it (if every
touched file is inside a whitelisted folder and the chain has no `rm` or
`mv`) or stashes it for review and raises the window. Reply is empty unless
a failure or a displaced chain happened; in those cases the reply is a
one-line report and the same sentence lands on the clipboard with a popup.

`run <file>` is a separate request kind (`RunFile`) that streams into the
same pane as `cargo`. Same transport, same frames, same summary. See the
`run` verb section below.

`overwrite <path>` followed by a newline, then the content. Kept for the old
one-step shape; internally becomes a one-step chain. Path must be absolute;
zapcli resolves `~` and relative paths before sending, so this only fires on
something else. Failures are the same shape as today: `zapgui: couldn't find
<path>`, `zapgui: no change to <path>`, `zapgui: overwrite needs an absolute
path: <path>`.

`focus` or `focus <anything>` — zapgui brings itself to the front, replies
empty. (Was called `replace`; renamed to free the word for the replace verb.)

`menu` — zapgui shows its general menu, replies empty.

Anything else — replies `zapgui: got "<request>"`.

Reply is read by zapcli after it shuts down its write half (EOF marker).

## Bringing the window to the front

`ViewportCommand::Focus` only sets keyboard focus on XFCE, it does not raise
the window. `wmctrl -a zapgui` does both. Called from the UI thread after
`Visible(true)`, because wmctrl on a hidden window is unreliable.

## Waking the UI thread

egui only runs the app's `ui` function when something asks it to. A hidden
window gets no input, so a hidden zapgui never runs `ui`, so it never learns a
chain arrived. Fix: the listener thread stores the egui Context at startup
(`set_ctx`), and calls `ctx.request_repaint()` after stashing a chain. That
wakes the UI, which reads the slot at the top of its frame. If the slot holds
a different chain than the one it last showed, it shows the window and raises
it. The slot is read, not taken; it holds the chain until confirm or deny, or
until a newer chain replaces it.

## Confirm and deny

The pending chain lives in one slot in terminal_router, as an `Arc<Chain>`
holding the steps, their diffs, and a snapshot of each touched file as it
was when the chain was built. The UI reads the slot every frame and renders
from the last Arc it saw. Glass buttons below the diff do the work: confirm
on the left, deny on the right.

Confirm applies every step that is not denied. If a step fails (its file
changed on disk since the snapshot, a create target already exists, a
replace old text no longer matches), nothing is written and the window
stays with a red error line, and the failure report goes on the clipboard
with a popup.

Deny marks the currently shown step denied, dims its block in the column on
the right, and moves to the next undecided step. When every step is denied
the window closes. Nothing is written.

A chain with more than one step shows a column of glass blocks on the right,
one per step. The shown step's block is brighter, denied blocks are dimmed,
and clicking a block shows that step. The caption above the diff says
`step n of m · verb path`, and appends ` · denied` when the shown step has
been denied.

On success the slot empties (only if it still holds the same Arc, so a newer
chain survives) and the window hides with `ViewportCommand::Visible(false)`,
not `Close`, so the process and its listener stay alive.

Clicks are ignored for 500 ms after the displayed diff changes, after a deny,
or after the window appears, so a click meant for something else can't land
on a freshly shown diff.

A new chain while one is showing replaces it. The reply and clipboard both
say `zap: replaced unconfirmed chain (<first path>).` so it doesn't
disappear silently.

## Reports and popups

Every decision (a step denied, Confirm, a failure, a whitelist auto-apply)
rebuilds a report: a plain sentence, one clause per file, saying what was
applied or denied or failed, with a bit of the diff as context. The report
goes on the clipboard, and a popup with a short label goes on screen. The
popup binary is `popup_demo_app`, invoked as
`popup_demo_app "text" x y host_id`; it draws a rounded box at x,y, sits
about 800 ms, fades, and exits. Popups carry short strings only.

## Whitelist

`~/.config/zap/whitelist.txt` (honours `XDG_CONFIG_HOME`), one absolute
folder per line. A chain whose touched files are all inside a whitelisted
folder is applied immediately — no diff, no window, just a
`Change was made` popup and the report on the clipboard. A chain with even
one file outside goes wholly to review.

A file counts as inside a folder after resolving symlinks and `..`, compared
by whole path components, not string prefixes. For a file that doesn't exist
yet, its nearest existing parent is used.

A chain with any `rm` or `mv` step goes to review even inside a whitelisted
folder, so an auto-applied edit can never be paired with an auto-applied
delete or rename.

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

Transport is framed, not end-of-file terminated: `[kind: u8][length: u32
LE][bytes]`. zapcli keeps its write half open so a cancel frame can go out
while output is flowing. One connection carries one request. A plain request
is request, reply, close. A run is CargoRun, then StreamOpen, StreamData...,
StreamDone.

Before every run, zapgui computes the project's execution surface — every
Cargo.toml, Cargo.lock, build script, proc-macro source, .cargo/config, and
rust-toolchain file that would run or change what runs during a build. If
the surface matches the trusted record for that project, the run starts
immediately. Otherwise a surface review window shows the new or changed
files and asks for a decision. Confirm stores the surface and starts;
Deny cancels the run.

Ordinary source files and `target/` are not in the surface, so editing the
program he is deliberately running never re-prompts. After a run, only
Cargo.lock's trusted copy is refreshed (cargo may have rewritten it). When a
chain is confirmed in review, any surface file it wrote in an already-trusted
project gets its trusted copy updated, because that review was the approval.
Whitelisted auto-applies do not do that.

## Line styles

Every command returns a `Vec<Line>`. `Line` has `text`, `style`, and
`skip_copy`. `LineStyle` is Normal, Error, Folder, Prompt, Zap, CliMessage,
Peek. The UI maps style to color. The UI never inspects line text to guess
what a line is — that's what the styles are for. If you add a command,
decide per line what it means.

Colors: Normal white, Error red, Folder yellow, Prompt dim green, Zap cyan,
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

## Rename-aware trust

A trust entry means: this exact content, at this path, with these args, was
approved. A rename through a confirmed `mv` step re-keys every entry whose
path was the source, keeping the same sha and text. At the next `run`, if
the content is unchanged, no prompt. If the content changed since approval,
the review shows a diff from the old approved text.

A `rm` step drops the entries for that path. Recreating a file at the same
path is new and needs a full review. A chain with any `rm` or `mv` skips
the surface content refresh — moved content never appeared as a diff, so
nothing is auto-trusted. Chains without `rm` or `mv` keep the old behaviour
of refreshing the cargo surface for files the chain wrote.

The RENAME REQUEST caption adds a `· trusted for running` note when the
source file is a run target or a file in a trusted cargo surface.

## rm, mv, mkdir

`rm`, `mv`, `mkdir` are chain steps. They have no marker body:

    rm <path> [<path>...]
    mv <from> <to>
    mkdir [-p] <path> [<path>...]

`rm` works on one text file at a time, and only ones it can keep an undo
copy of. It refuses directories. `mv` never overwrites: to replace a file,
`rm` it first in the same chain. `mkdir` accepts `-p` and always creates
parents.

A chain with any `rm` or `mv` step always goes to review, even inside a
whitelisted folder. The DELETE REQUEST or RENAME REQUEST banner is drawn
above the caption. Confirm refuses to apply a chain if an `rm` or `mv` step
was never shown at least once; it jumps to the first such step and says so.
Deny once and the guard is satisfied for that step.

## The Ctrl+C problem

egui-winit converts Ctrl+C to `Event::Copy` regardless of whether Shift is
held (`is_copy_command` only checks `modifiers.command`). That means Ctrl+C
and Ctrl+Shift+C are indistinguishable by the time events reach the app. To
work around this, zapcli's `raw_input_hook` strips both the Key(C) event and
any `Event::Copy` before egui processes them. It reads modifiers from the Key
event when available and falls back to ClearInput when only `Event::Copy` came
through. The `copy` command is the reliable way to copy everything.

## The input line

The input is an `egui::TextEdit` (multiline, one row tall, frame off). It
gives Ctrl+Z, Ctrl+Y, selection, Ctrl+A, cut and click-to-place cursor
without extra code. Plain Enter submits; Shift+Enter inserts a newline; Tab
is stripped in `raw_input_hook` so it can't move focus to a pane button.
Focus is requested at most once per frame and only when nothing else holds
it — requesting focus every frame spins the render loop.

## Where files go

zapgui's `app_core/src/chain/` — the chain module. step.rs (step types),
parser.rs (chain text -> steps), play.rs (play forward in memory), writer.rs
(the one routine that writes), report.rs (clipboard / popup sentences),
whitelist.rs (list file, matching), popup.rs (popup_demo_app wrapper).

zapgui's `app_core/src/terminal_router/` — socket listener, route_requests,
and the shared slot in mod.rs. The slot holds an `Arc<Chain>`.

zapgui's `app_core/src/clipboard.rs` — the xclip helper, returning an error
string when xclip can't start or exits non-zero.

zapgui's `every_page_ever/src/ReusableComponents/` — close_button, glass_button,
diff_viewer. Each component has a `mod.rs` and `ui.rs`.

zapcli's `app_core/src/command_router/` — dispatch, plus `core_commands/<name>.rs`
per verb. `core_commands/chain.rs` forwards create, replace, overwrite, rm,
mv, and mkdir; the cargo and run verbs live inline in `mod.rs` because they
open streaming sessions.

`zapcli/app_core/src/general_util/socket.rs` — the framed client: `open` for
plain requests, `open_cargo` for a streaming run. Connect and auto-launch
for a run happen on the reader thread, so a slow launch never blocks the UI.

`zap/cli/app/src/egui_setup/drawing_requirements.rs` — the terminal UI, the
stream pane, `raw_input_hook` for Ctrl+C/Ctrl+Shift+C.

`zapcli/app_core/src/general_util/socket.rs` — `open` for plain requests,
`open_cargo` and `open_run` for streaming sessions. Connect and auto-launch
for a streaming request happen on the reader thread, so a slow launch never
blocks the UI.

`zapgui/app_core/src/last_change.rs` — the in-memory record of the last
change per file.

`zapgui/app_core/src/run_trust.rs` — run-target trust entries, one per
(canonical path, exact args), stored under `~/.config/zap/run_trust/`.

`zapgui/app_core/src/terminal_router/stream_run.rs` — the shared process
loop both cargo and run use.

`zapgui/app_core/src/terminal_router/run_file.rs` — the run verb's server
side.

zapcli's `app_core/src/shortcuts/` — one file per shortcut, plus mod.rs to
dispatch.

zapcli's `app/src/egui_setup/drawing_requirements.rs` — the actual terminal UI.

zapgui's `app/src/egui_setup/drawing_requirements.rs` — the actual diff UI.

## Adding a command

Create `zapcli/app_core/src/command_router/core_commands/<name>.rs`. Write
`pub fn run(args: &[&str], session: &Session) -> Outcome`. Add it to
`core_commands/mod.rs`. Add a match arm in `command_router/mod.rs`. Return
`Vec<Line>` in `Outcome::Print` — pick a `LineStyle` per line.

If the command needs to forward a multi-line block to zapgui, take the
verbatim input (not `trimmed`) and send it as `chain <cwd>\n<block>`. See
`core_commands/chain.rs` for the existing create and replace verbs.

## Build

`~/ProgStuff/egui/zap/build_both.sh`. Builds and installs `zapgui`, `zapcli`,
`zap-settings`, and the `zap` wrapper (`~/.local/bin/zap`, a bash script that
execs `zapcli`). It also kills a running zapgui and starts a fresh hidden
one, so a rebuild takes effect immediately.

`zap_wire/` is a small crate both binaries depend on: the framed-message
types and the cargo allow-list. It lives beside `zapgui/` and `zapcli/` as
a sibling workspace.

## Socket path

`/run/user/1000/zap.sock`. Hardcoded in two places:
`zapgui/app_core/src/terminal_router/mod.rs` as `SOCKET_PATH`, and
`zapcli/app_core/src/general_util/socket.rs` as `SOCKET`. If your user id is
not 1000, both need changing.
