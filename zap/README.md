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

Two request shapes:

`overwrite <path>` followed by a newline, then the content. Path is on the
first line, content is everything after that first newline. Content can be
multiline (paste in zapcli preserves newlines). zapgui reads the file at path,
diffs old vs new, stashes the diff, replies empty. If the file can't be read,
replies `zapgui: couldn't find <path>`.

`replace` or `replace <anything>` — zapgui brings itself to the front, replies
empty.

Anything else — replies `zapgui: got "<request>"`.

Reply is read by zapcli after it shuts down its write half (EOF marker).

## Bringing the window to the front

`ViewportCommand::Focus` only sets keyboard focus on XFCE, it does not raise
the window. `wmctrl -a zapgui` does both. Called from the UI thread after
`Visible(true)`, because wmctrl on a hidden window is unreliable.

## Waking the UI thread

egui only runs the app's `ui` function when something asks it to. A hidden
window gets no input, so a hidden zapgui never runs `ui`, so it never learns a
diff arrived. Fix: the listener thread stores the egui Context at startup
(`set_ctx`), and calls `ctx.request_repaint()` after stashing a diff. That
wakes the UI, which picks up the diff, shows the window, and raises it.

## Line styles

Every command returns a `Vec<Line>`. `Line` has `text` and `style`. `LineStyle`
is Normal, Error, Folder, Prompt, Zap, CliMessage. The UI maps style to color.
The UI never inspects line text to guess what a line is — that's what the
styles are for. If you add a command, decide per line what it means.

Colors: Normal white, Error red, Folder yellow, Prompt dim green, Zap cyan,
CliMessage gray.

## The Ctrl+C problem

egui-winit converts Ctrl+C to `Event::Copy` regardless of whether Shift is
held (`is_copy_command` only checks `modifiers.command`). That means Ctrl+C
and Ctrl+Shift+C are indistinguishable by the time events reach the app. To
work around this, zapcli's `raw_input_hook` strips both the Key(C) event and
any `Event::Copy` before egui processes them. It reads modifiers from the Key
event when available and falls back to ClearInput when only `Event::Copy` came
through. The `copy` command is the reliable way to copy everything.

## Where files go

zapgui's `app_core/src/terminal_router/` — socket listener and route_requests.

zapgui's `every_page_ever/src/ReusableComponents/` — close_button, glass_button,
diff_viewer. Each component has a `mod.rs` and `ui.rs`.

zapcli's `app_core/src/command_router/` — dispatch, plus `core_commands/<name>.rs`
per verb.

zapcli's `app_core/src/general_util/` — things that aren't about commands:
path_parsing, socket.

zapcli's `app_core/src/shortcuts/` — one file per shortcut, plus mod.rs to
dispatch.

zapcli's `app/src/egui_setup/drawing_requirements.rs` — the actual terminal UI.

zapgui's `app/src/egui_setup/drawing_requirements.rs` — the actual diff UI.

## Adding a command

Create `zapcli/app_core/src/command_router/core_commands/<name>.rs`. Write
`pub fn run(args: &[&str], session: &Session) -> Outcome`. Add it to
`core_commands/mod.rs`. Add a match arm in `command_router/mod.rs`. Return
`Vec<Line>` in `Outcome::Print` — pick a `LineStyle` per line.

## Build

`~/ProgStuff/egui/zap/build_both.sh`. Builds and installs `zapgui`, `zapcli`,
`zap-settings`, and the `zap` wrapper (`~/.local/bin/zap`, a bash script that
execs `zapcli`).

## Socket path

`/run/user/1000/zap.sock`. Hardcoded in two places:
`zapgui/app_core/src/terminal_router/mod.rs` as `SOCKET_PATH`, and
`zapcli/app_core/src/general_util/socket.rs` as `SOCKET`. If your user id is
not 1000, both need changing.
