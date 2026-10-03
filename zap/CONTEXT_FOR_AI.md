# Context for a fresh AI

You've been dropped into the zap workspace. Read in this order. Don't read the
whole repo, most of it is noise.

## 1. The top-level shape

Read `README.md` first (the one next to this file). It has the architecture,
the wire protocol, the egui patch, and the parts that will confuse you if you
don't read it.

## 2. What zapgui is doing

`zapgui/app_core/src/terminal_router/mod.rs` — socket state and shared diff
slot. Small.

`zapgui/app_core/src/terminal_router/listener.rs` — reads the whole stream,
hands to route, writes reply.

`zapgui/app_core/src/terminal_router/route_requests.rs` — parses the request
shape.

`zapgui/app/src/egui_main.rs` — startup, icon, listener spawn, ctx storage.

`zapgui/app/src/egui_setup/drawing_requirements.rs` — the actual diff UI.

## 3. What zapcli is doing

`zapcli/app_core/src/command_router/mod.rs` — dispatch. Look at this to know
what verbs exist.

`zapcli/app_core/src/command_router/core_commands/mod.rs` — the list of
command files.

`zapcli/app_core/src/general_util/path_parsing.rs` — how `~`, `~/...`,
absolute, and relative paths get resolved.

`zapcli/app_core/src/general_util/socket.rs` — how zapcli talks to zapgui.

`zapcli/app_core/src/shortcuts/mod.rs` and its children — Ctrl+C and
Ctrl+Shift+C handling.

`zapcli/app/src/egui_setup/drawing_requirements.rs` — the terminal UI, the
`raw_input_hook`, the event loop.

## 4. The diff engine

`zapgui/app_core/src/text_diff/` — `pair_lines` produces raw rows, `process`
groups them into blocks the UI can render. This is what zapgui's UI consumes.

## 5. Reusable UI

`zapgui/every_page_ever/src/ReusableComponents/` — close_button, glass_button,
diff_viewer.

## 6. The egui patch

`egui-src/crates/eframe/src/native/epi_integration.rs` — one function is
patched. Everything else in `egui-src` is stock 0.36.2. Don't touch anything
else in there unless you know why.

## 7. Don't

Don't `cargo update`. Don't change the egui version. The patch depends on the
exact source.

Don't add heuristic line-coloring to zapcli's UI. Use `LineStyle`.

Don't add new shortcut keys without reading the Ctrl+C section in `README.md`
— egui-winit steals some combinations before the app ever sees them.

Don't assume zapcli can reach a real shell. It can't. That's the point.
