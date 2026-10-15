# Notes for the next AI

## How the user works

- Short answers. Every sentence should add information.
- Prose, not bullets, unless they ask for bullets.
- Explain one thing at a time. If you have more, ask first.
- Take them literally.
- No metaphors. No dumbed-down versions. Plain words, no jargon.
- One paste per task. Combine commands. They hate doing things one at a
  time.

## How they edit code

Whole-file rewrites via `cat > file << 'EOF'`. Targeted edits via Python
heredoc that asserts the old text is present before replacing. Never sed
or perl. Never nest heredocs. Absolute paths from `$HOME`.

They prefer zap chains when the change fits the grammar. `replace`,
`overwrite`, `append`, `create` with markers. A chain with any `zaprm` or
`zapmv` always goes to review, so a same-file edit that also deletes is two
steps in one chain and one review window.

When a chain has many fiddly edits, use Python heredoc instead. The
matcher in Python is exact; the parser in a chain is line-oriented and
weaker at whitespace.

## Building

`~/ProgStuff/egui/zap/build_both.sh` builds zapgui, zapcli and zap-settings,
installs them to `~/.local/bin`, refreshes the `zap` wrapper, and restarts
zapgui hidden. End any command block with `build_both.sh && zap` or
`build_both.sh && zapgui` so they don't have to launch it manually.

From the zap root, `c t` runs `cargo test` in zapgui, zapcli and zap_wire,
one after the other, printing a header per workspace. `c c` for check,
`c b` for build. Inside a workspace, `c` falls through to plain cargo.

The `RUSTFLAGS` set by `build_both.sh` and by the root `local_cargo.sh`
overrides the cranelift rustflags in `~/.cargo/config.toml`. That's why
fontdue compiles. Don't remove it.

## What's done

- zapcli with ls, cd, cat (multi-file, `=== path ===` headers), settings,
  exit/quit, cls/clear, copy, copy all, zap, overwrite, create, replace,
  zaprm, zapmv, mkdir, append, head, tail, wc, last, help, whitelist, cargo,
  run.
- Chains: create/replace/overwrite/append/zaprm/zapmv/mkdir blocks that play
  forward in memory and are written all-or-nothing. See
  `CHAIN_CHEATSHEET.md`.
- Shared parser in `zap_wire::chain`. Both zapcli and zapgui call it, so
  the absolute-path rule and the cargo allow-list are enforced the same
  way on both sides.
- Absolute-path rule on chain writes: relative paths are rejected in
  zapcli before anything leaves the socket.
- Marker escape: `@@new` / `@@old` / `@@end` at the start of a content
  line means one literal `@`. Only lines that collide with a marker word
  are escapes; `@@ -1,3 +1,4 @@` and `@derive(Debug)` pass through.
- `append` verb with `--no-newline` on `create` and `append`.
- Run steps in a chain: a `cd <folder>` line followed by a `cargo <args>`
  or `run <file> [args]` line, at most one per chain, and it must be the
  last step. Nothing runs until the human confirms. The connection
  (`FrameKind::ChainRun`) stays open through review, the writes and the
  run, so the pane streams output the whole time.
- zapgui diff view with per-step review: caption, glass-block column for
  picking a step, confirm and deny below.
- Word wrap in the viewer, per-side bar colours (red left / green right,
  violet on a move), a bright arrow marking the continuation row of a
  wrapped line.
- Select-and-copy text inside the viewer: rows are selectable labels,
  Ctrl+C works.
- Full-file toggle: a pill labelled "full file" / "changes only" that
  re-pairs the step's before/after without the 3-line context limit.
- Move detection: a run of deleted lines matched against a run of
  inserted lines, shown as a violet bar plus a labelled strip above the
  span. Runs across the whole chain, so a move between two files is
  caught too. Thresholds are first guesses; see `text_diff/moves.rs`.
- Edit-vs-replace: paired old/new lines use token similarity. A small
  change is a `Change` row; an unrelated swap reads as `Delete` then
  `Insert`.
- Failure reports name the step index and verb.
- Deny turns the step's block red; if that step is also the shown one, a
  brighter red.
- Popups wired to the `fading_popup` binary in
  `~/ProgStuff/egui/reusable_components`. `build_both.sh` builds and
  installs it.
- Distinct banners for RUN / EXECUTE / DELETE / RENAME requests.
- Mis-click guard: Confirm refuses to apply a zaprm, zapmv, cargo or run
  the user never looked at.
- Whitelist: `~/.config/zap/whitelist.txt` and every chain that only
  touches files inside it applies silently, unless it has rm, mv, cargo
  or run.
- Menu on a normal zapgui launch; `--hidden` for the auto-launch path.
- Single-instance: a manual second launch re-shows the menu on the
  running one instead of stealing its socket.
- egui patched so hidden-at-startup doesn't flash.
- Cargo and run stream into a bottom pane, with the progress bar drawn.
- Run targets are trusted per (path, args, content hash); rename through
  a confirmed mv carries the trust.
- `last` shows the last change zap made to a file.
- head/tail/wc/last output is peek-only; copy skips it.
- Input line is an egui TextEdit: Ctrl+Z, click-to-position, selection.
- Ctrl+C clears input, Ctrl+Shift+C is unreliable (see README).
- Both projects under `~/ProgStuff/egui/zap/`.

## What's spec'd but not built

In `todo.md` and `todo_later.md`. Highlights:

- `find`, `grep`, `rg` under the hood.
- git read-only: status, log, diff, show.
- fzf-style pickers replaced with something (`cdf`, `s`, `b`, `co`, `cop`).
- `t` (tree to clipboard), `cur` (pwd to clipboard).
- `chmod +x` as a chain step, always reviewed, with an EXEC REQUEST
  banner.
- A chain that mixes auto-apply-safe steps with one rm/mv should only
  ask for approval on the rm/mv step, not the whole chain.
- `last` should give diff plus current contents, and copy to clipboard
  by default.
- Word wrap on the input line in zapcli is wrong: pasting wraps but
  continuation lines start indented; typing runs past the right edge.

## Things that bit us

- `wmctrl -a` is the only thing that raises the window on XFCE.
  `ViewportCommand::Focus` only sets keyboard focus.
- wmctrl on a hidden window silently does nothing. Make visible first.
- A hidden window gets no egui ticks. Call `ctx.request_repaint()` after
  stashing anything for the UI.
- `ui.max_rect()` is in points, not pixels. On HiDPI it can be smaller
  than you expect.
- `add_sized` centers its widget. Use `allocate_ui_with_layout` with
  left_to_right if you want left alignment.
- `TextWrapMode::Wrap` on a Label allocates full width. That's what you
  want for whole-row selection hit-testing, and it's why the viewer's
  labels use it.
- egui's `LabelSelectionState` constrains a drag to labels that overlap
  the selection's x-range. Left and right column labels have disjoint
  x-ranges, so a drag stays inside one column even when the labels are
  added interleaved. Don't assume draw order matters for that.
- eframe unconditionally calls `window.set_visible(true)` after the first
  frame. Tracked as egui issue #3654. Only patchable.
- egui-winit converts Ctrl+C to Event::Copy regardless of Shift. Tracked
  as issue #4065. The `raw_input_hook` workaround in zapcli is the only
  way to intercept.
- `mpsc::Sender` is Send but not Sync. A `Chain` stored inside an `Arc`
  cannot hold one bare; wrap it in a `Mutex`.
- `LayoutJob` and `TextWrapping` live at `egui::text::`, not at the crate
  root.
- `similar::TextDiffable` is not a public trait. Inline the pairing body
  in both `pair_lines` and `pair_lines_full` rather than generic over
  the diff type.

## A stylistic thing

Every command returns a `Vec<Line>` where each Line knows its own style.
The UI never inspects line text to decide colour. If you add a command,
pick a LineStyle per line. Don't guess from content.