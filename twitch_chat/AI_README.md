# twitch_chat — AI context

Project root: /home/zakke/ProgStuff/egui/twitch_chat
What it is: a transparent, click-through, always-on-top Twitch IRC chat overlay for X11.
Workspace: four members — `app` (bin name `chat`, default member), `core` (crate name `notes_core`), `history_viewer`, `twitch_ban`.

## Build

- `./local_cargo.sh` or `c run` from root — sets RUSTFLAGS="-C link-arg=-fuse-ld=mold -Zcodegen-backend=llvm", defaults to `cargo run`. Don't invoke plain `cargo` — the user's global toolchain differs and egui doesn't build under it.
- `default-members = ["app"]` in workspace Cargo.toml — building the default only builds `chat`. Use `./local_cargo.sh build -p history_viewer` to build the viewer.
- `./build_and_install.sh` — release build, packages chat_0.1.0_amd64.deb, dpkg-installs as `chat`.
- Don't run `cargo build`/`cargo check` unless it's necessary. Assume builds pass unless told otherwise.

## Hotkeys (X11 grabs on root)

- Super+Insert — toggle. If history_viewer is not running, snapshots the last 30 messages to /tmp/twitch_chat_history.json and spawns the viewer. If it is running, SIGKILLs it. The overlay holds the child handle and reaps it in `ui()` via `try_wait` when the viewer exits on its own (window closed). No reaper thread.
- Super+Delete — quits the app
- Super+Alt+Delete — hides/shows the overlay content

## Files

- `app/src/main.rs` — entry. One call: `egui_main::run()`.
- `app/src/lib.rs` — module list (egui_main, egui_setup, hotkeys, twitch, x11_overlay).
- `app/src/egui_main.rs` — sets up the hotkey channel, spawns the hotkey thread, calls x11_overlay::apply(), gets options from egui_setup::config, runs eframe.
- `app/src/egui_setup/config.rs` — returns the eframe::NativeOptions (transparent, undecorated, always-on-top, Dock window type, mouse-passthrough flag, 400x600 at 10,10, resizable).
- `app/src/egui_setup/drawing_requirements.rs` — the `App` struct and its `impl eframe::App`. Owns the message VecDeque, the cmd receiver, draws the live view. Holds `spawn_history` (serializes to /tmp/twitch_chat_history.json, spawns `history_viewer` from the same dir as the current exe, reaps the child in a background thread).
- `app/src/egui_setup/post_init_config.rs` — exists for structural parity with the egui_template layout; currently empty. Would hold &egui::Context-dependent closures if needed.
- `app/src/hotkeys.rs` — grabs keys on root, sends `Cmd` over mpsc. Cmd variants: SpawnHistory, ToggleHidden, Quit.
- `app/src/twitch.rs` — spawns a thread with a tokio current-thread runtime, joins #zakkeakke, forwards (user, text) over mpsc, calls ctx.request_repaint() per message.
- `app/src/x11_overlay.rs` — the click-through hack. Polls root's children every 100ms for a window with _NET_WM_NAME = "chat". When found: zeroes _GTK_FRAME_EXTENTS, sets _NET_WM_STATE_ABOVE, sets override_redirect, applies an empty XFixes INPUT shape, reparents to root at (10, 10). This sequence is load-bearing as a whole — a sleep between steps broke it, and individual steps were never verified in isolation. Treat any change here as high-risk, and test click-through after every change.
- `history_viewer/src/main.rs` — a normal eframe window. Reads a JSON path from argv, deserializes `Vec<notes_core::Message>`, renders oldest-to-newest inside a ScrollArea with stick_to_bottom(true). No transparency, no click-through, no X11 machinery — this is why it can scroll and hover works. Each row has a hover-only ban button on its right; clicking it opens an inline `egui::Modal` confirm popup (Confirm/Cancel, Esc cancels unless a ban is in flight). Confirming spawns a thread that calls `twitch_ban::ban_user_permanently`, sends the result back over mpsc, and the modal shows a spinner while in flight or red error text plus Retry/Cancel on failure. On success the modal switches to a "Banned" heading with a Close button.
- `twitch_ban/src/lib.rs` — dependency of `history_viewer` only; `app` does not depend on it. Public surface is one function: `ban_user_permanently(target_user_id: &str) -> Result<(), BanError>`. Reads `~/.config/twitch_chat/secrets.env` itself each call, no caching. Three blocking HTTP calls per invocation via `ureq`: POST `id.twitch.tv/oauth2/token` to refresh the access token (rewrites the `TWITCH_REFRESH_TOKEN=` line in secrets.env in place if Twitch rotates it), GET `api.twitch.tv/helix/users` to resolve the token owner's own user ID (used for both broadcaster_id and moderator_id), POST `api.twitch.tv/helix/moderation/bans` with `{"data":{"user_id": target}}` to ban. No duration field — permanent. Error enum: `Secrets`, `Refresh`, `ResolveSelf`, `Ban`, each carrying a descriptive string with HTTP status and body when available.
## Known issues

- None currently. Black box fix (skip draw when no visible messages) is in place. History viewer + scroll works. Zombie process accumulation from repeated spawns is fixed by a reaper thread.

## Behaviors

- Live view (default): fades older messages. Messages older than FADE_AFTER + FADE_DURATION are hidden. Cap VISIBLE_MAX=10 shown. Skips drawing entirely if nothing visible, so the black background frame doesn't render as a tiny square.
- Message storage: VecDeque capped at 30 (MAX_MESSAGES). Oldest dropped when exceeded.
- Hover-hide: overlay content is hidden while pointer is over it. Applies only to the overlay.
- Super+Insert: overlay is not involved — it fires a separate process. Snapshot includes up to the last 30 messages (whatever is in the VecDeque).


## Secrets file

`~/.config/twitch_chat/secrets.env`, chmod 600, outside the git repo. Three lines: `TWITCH_CLIENT_ID`, `TWITCH_CLIENT_SECRET`, `TWITCH_REFRESH_TOKEN`. Obtained once via `twitch token -u -p 8123 -s moderator:manage:banned_users`. Twitch app redirect URL is `http://localhost:8123`. The refresh token line can be rewritten in place by `twitch_ban` when Twitch rotates it — the file's contents are not static. If the refresh token dies, the twitch-cli flow has to be re-run and the line re-pasted by hand.

## Rules the user expects

- No unwrap() or unwrap_or_default() unless the default is genuinely the intended behavior at that spot.
- Errors carry err_msg, method, filepath. Propagate with ? where the signature allows; otherwise use the project's unwrap_or_bail!() if present, else log with the same three things and continue. Never silently discard an error.
- Don't refactor the user's code. Only code the AI wrote may be refactored. If something looks wrong, say so and let the user decide.
- Don't add unrequested features. If a task needs something else built first, stop and ask.
- Don't rename existing things.
- Answers should be short. Prose, not bullet lists, unless explicitly asked.
