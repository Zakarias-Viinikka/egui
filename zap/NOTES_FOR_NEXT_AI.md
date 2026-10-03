# Things that aren't in the source yet

## How the user wants you to work

Read the memory transfer doc if you have it. If you don't, the short version:

- Short answers. Every sentence should add information.
- Prose, not bullets, unless they ask for bullets.
- Explain one thing at a time. If you have more, ask first.
- Take them literally. When they ask "what is X", answer what X is.
- No metaphors. No dumbed-down versions. Plain words, no jargon.
- If they say "you're doing the thing", don't rephrase. Try a different shape.
- One paste per task. Combine commands. They hate doing things one at a
  time, and they've said so explicitly, twice.

## How they edit code

Whole-file rewrites via `cat > file << 'EOF'`. Targeted edits via python
heredoc that asserts the old text is present before replacing. Never sed or
perl. Never nest heredocs. Absolute paths from `$HOME`.

## The delivery pattern

They run `~/ProgStuff/egui/zap/build_both.sh` after every change. That builds
zapgui and zapcli, installs them to `~/.local/bin`, and refreshes the `zap`
wrapper. The command block usually ends with `build_both.sh && zap` or `&&
zapgui` so they don't have to launch it manually.

## What's done

- zapcli with ls, cd, cat, settings, exit/quit, cls/clear, copy, zap,
  overwrite
- zapgui showing a diff view of overwrite requests
- zapgui starts hidden, wakes on overwrite, brings itself to front via wmctrl
- egui patched so hidden-at-startup doesn't flash
- Ctrl+C clears input, Ctrl+Shift+C is unreliable (see README)
- Both projects organized under ~/ProgStuff/egui/zap/

## What's spec'd but not built

In `zapcli/plan.md`:

- head, tail, find (uses rg), grep (uses rg), rg, wc
- git read-only: status, log, diff, show
- fzf-pickers replaced with something (cdf, s, b, co, cop, run)
- t (tree to clipboard)
- cur (pwd to clipboard)
- Two whitelists: executables (safe-to-run), and files/folders (AI can edit
  without approval). Not started.

## What "approve" means in zapgui

Not decided. The diff viewer has three glass buttons on the right that do
nothing yet. The user hasn't said what they're for. Don't guess.

## The egui patch

`egui-src/crates/eframe/src/native/epi_integration.rs`, in `post_rendering`.
The line `window.set_visible(true)` on the first frame was removed. If it ever
comes back (e.g. re-clone from upstream), zapgui will flash on startup.

## Things that bit us

- `wmctrl -a` is the only thing that raises the window on XFCE. `ViewportCommand::Focus`
  only sets keyboard focus.
- wmctrl on a hidden window silently does nothing, so make visible first.
- A hidden window gets no egui ticks. Must call `ctx.request_repaint()` after
  stashing anything for the UI.
- `ui.max_rect()` is in points, not pixels. On HiDPI it can be smaller than
  you expect.
- `add_sized` centers its widget. Use `allocate_ui_with_layout` with
  left_to_right if you want left alignment.
- `TextWrapMode::Wrap` on a Label makes it allocate full width, which is what
  you need for whole-row selection hit-testing.
- eframe unconditionally calls `window.set_visible(true)` after the first
  frame. Tracked as egui issue #3654. Only patchable.
- egui-winit converts Ctrl+C to Event::Copy regardless of Shift. Tracked as
  issue #4065. The `raw_input_hook` workaround in zapcli is the only way to
  intercept.

## A stylistic thing

The user named the design pattern after the fact: every command returns a
`Vec<Line>` where each Line knows its own style. The UI never inspects line
text to decide color. They were very clear about this. If you add a command,
pick a LineStyle per line. Don't guess from content.
