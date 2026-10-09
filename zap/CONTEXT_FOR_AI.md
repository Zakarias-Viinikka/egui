# Context for a fresh AI

You've been dropped into the zap workspace. Read in this order. Don't read the
whole repo, most of it is noise.

## 1. The top-level shape

Read `README.md` first (the one next to this file). It has the architecture,
the wire protocol, the egui patch, and the parts that will confuse you if you
don't read it.

## 2. What zapgui is doing

`zapgui/app_core/src/chain/` — the chain module. step.rs (step types),
parser.rs (chain text -> steps), play.rs (play forward in memory),
writer.rs (the one routine that writes), report.rs (clipboard / popup
sentences), whitelist.rs (list file, matching), popup.rs (popup_demo_app
wrapper).

`zapgui/app_core/src/terminal_router/mod.rs` — socket state and the shared
pending-chain slot. Small.

`zapgui/app_core/src/terminal_router/listener.rs` — reads the whole stream,
hands to route, writes reply.

`zapgui/app_core/src/terminal_router/route_requests.rs` — parses the request
shape, dispatches to chain or overwrite, auto-applies whitelisted chains.

`zapgui/app_core/src/clipboard.rs` — xclip wrapper.

`zapgui/app/src/egui_main.rs` — startup, icon, listener spawn, ctx storage,
single-instance handoff.

`zapgui/app/src/egui_setup/drawing_requirements.rs` — the review window and
the general menu.

## 3. What zapcli is doing

`zapcli/app_core/src/command_router/mod.rs` — dispatch. Look at this to know
what verbs exist. `create` and `replace` both forward their multi-line block
as a chain; `core_commands/chain.rs` is the shared forwarder.

`zapcli/app_core/src/command_router/core_commands/mod.rs` — the list of
command files.

`zapcli/app_core/src/general_util/path_parsing.rs` — how `~`, `~/...`,
absolute, and relative paths get resolved.

`zapcli/app_core/src/general_util/socket.rs` — how zapcli talks to zapgui.

`zapcli/app_core/src/shortcuts/mod.rs` and its children — Ctrl+C and
Ctrl+Shift+C handling.

`zapcli/app/src/egui_setup/drawing_requirements.rs` — the terminal UI, the
`raw_input_hook`, the event loop.

## 4. Chain text format

`CHAIN_CHEATSHEET.md` in the repo root. Read it if you're touching the
parser or writing an example chain.

## 5. The diff engine

`zapgui/app_core/src/text_diff/` — `pair_lines` produces raw rows, `process`
groups them into blocks the UI can render. This is what zapgui's UI consumes.

## 6. Reusable UI

`zapgui/every_page_ever/src/ReusableComponents/` — close_button, glass_button,
diff_viewer.

## 7. The egui patch

`egui-src/crates/eframe/src/native/epi_integration.rs` — one function is
patched. Everything else in `egui-src` is stock 0.36.2. Don't touch anything
else in there unless you know why.

## 8. Don't

Don't `cargo update`. Don't change the egui version. The patch depends on the
exact source.

Don't add heuristic line-coloring to zapcli's UI. Use `LineStyle`.

Don't add new shortcut keys without reading the Ctrl+C section in `README.md`
— egui-winit steals some combinations before the app ever sees them.

Don't assume zapcli can reach a real shell. It can't. That's the point.
