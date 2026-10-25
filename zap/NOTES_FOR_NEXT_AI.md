# Notes for the next AI

Last updated: 2026-10-25

## How the user works

Short answers. Every sentence adds information. Prose, not bullets.
Explain one thing at a time; if you have more, ask first. Take them
literally. No metaphors, no jargon, no dumbed-down versions. One paste
per task; they hate doing things one at a time.

## How they edit code

Whole-file rewrites via `cat > file << 'EOF'`. Targeted edits via Python
heredoc that asserts the old text exists before replacing. Never sed,
never perl -i, never nested heredocs. Absolute paths from $HOME.

Prefer zap chains when the change fits the grammar. Python heredoc when
many fiddly edits - the matcher is exact, the chain parser is
line-oriented.

## Building

~/ProgStuff/egui/zap/build_both.sh builds zapgui, zapcli, zap-settings,
the zap wrapper and fading_popup, restarts zapgui hidden. End command
blocks with `build_both.sh && zap`.

c t / c c / c b from the zap root test / check / build all three
workspaces. Inside a workspace c falls through to plain cargo.

The RUSTFLAGS set by build_both.sh and local_cargo.sh override the
cranelift rustflags in ~/.cargo/config.toml - that's why fontdue
compiles. Don't remove them.

## What's done

- zapcli verbs: ls, cd, cat (multi-file, === path === headers),
  s / settings, exit / quit, cls / clear, copy, copy all, zap,
  overwrite, create, replace, zaprm, zapmv, mkdir, rmdir, append, head,
  tail, wc, last, retry, help, whitelist, cargo, run.
- Chains play forward in memory, written all-or-nothing. Shared parser
  in zap_wire::chain. Absolute-path rule. Marker escape @@. --no-newline
  on create / append.
- Run steps: cd + cargo or run, at most one, must be last.
  FrameKind::ChainRun keeps the connection open through review, writes
  and run.
- Diff view: three-pass renderer, word wrap, per-side bars, selectable
  text, full-file toggle, one-line edit as one row, HR above any non-
  Equal block, pane-local selection.
- Move detection across the whole chain. Snippet view is Dimmed (source
  rows dim grey, no strip; same-step destinations paired Change rows).
  Full-file and surface reviews are Classic.
- Review window: Verify / Mark / Allow / Deny, 150 ms click guard in
  both windows, tooltips everywhere, boundary chip on the caption row.
  Verb-tinted blocks. Distinct banners per verb.
- Reports: sentence per decision, on the clipboard, popup label.
- Whitelist in ~/.config/zap/whitelist.txt; rm / mv / cargo / run /
  unapproved .sh / .toml always go to review. FILE_TRUST.md.
- egui patched so hidden-at-startup doesn't flash.
- Single-instance: second launch re-shows the menu on the running one.
- Run targets trusted per (path, args, content hash); rename through a
  confirmed mv carries the trust.
- last shows diff plus current contents, copies to clipboard, peek-only
  so copy skips it. head / tail / wc same skip rule.
- Input line is an egui TextEdit: Ctrl+Z, selection, click-to-place.
  Ctrl+C clears input; Ctrl+Shift+C is unreliable (see README).
- retry [<step>...] rebuilds the last review's unapplied steps.

## What's not built

In todo.md and todo_later.md. Highlights: find / grep / rg, git
read-only, fzf-style pickers (cdf, s, b, co, cop), t / cur, chmod +x as
a chain step, partial-chain approval (only ask about the rm / mv step),
input-line word wrap (continuation indent + overrun).

## Things that bit us

- wmctrl -a is the only thing that raises the window on XFCE.
  ViewportCommand::Focus only sets keyboard focus. On a hidden window
  wmctrl silently does nothing - make visible first.
- A hidden window gets no egui ticks. Call ctx.request_repaint() after
  stashing anything for the UI.
- ui.max_rect() is points, not pixels. HiDPI surprises.
- add_sized centres its widget. allocate_ui_with_layout + left_to_right
  for left alignment.
- TextWrapMode::Wrap on a Label allocates full width - needed for
  whole-row selection.
- eframe unconditionally calls window.set_visible(true) after the first
  frame (egui issue 3654). Only patchable.
- egui-winit turns Ctrl+C into Event::Copy regardless of Shift (issue
  4065). raw_input_hook in zapcli is the only interception point.
- mpsc::Sender is Send but not Sync. Chain inside an Arc needs the
  Sender in a Mutex.
- LayoutJob and TextWrapping live at egui::text::.
- similar::TextDiffable is not public. Inline the pairing body in
  pair_lines and pair_lines_full.

## Where the pending decisions live

implementation_plan_2026-10-16.md is the source of truth for the next
batch. Its header is the session contract; steps 1-3 are the format
template; step 4 onward are the settled designs. When a step's design
turns out wrong at implementation time, amend the step in the plan; do
not restate it here.