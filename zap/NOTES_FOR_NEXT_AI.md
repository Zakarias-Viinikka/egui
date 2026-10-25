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
- Edit-vs-replace split into two thresholds: SIMILARITY_THRESHOLD 0.5
  pairs, EDIT_THRESHOLD 0.6 draws blue bars. Change rows carry
  `edit` and per-side `changed` ranges from `inline_spans`, and the
  viewer highlights only those bytes. No spots when either line is over
  2,000 chars or the marks cover >70% of the non-space characters.
- Syntax coloring for Rust and TOML in
  `diff_viewer/syntax.rs` (syntect + two-face). Tables are built once
  per side, cached in NecessaryStructForEgui, theme re-read each time.
  `off` and unknown extensions give empty tables that draw plain.
- `segments_for_line(len, runs, highlights)` in `diff_viewer/ui.rs` is
  the pure helper pass 0 uses to build each row's LayoutJob. Highlight
  ranges only add cut points; the segments carry no "highlighted" flag.
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
- Step 6: `file_trust` key is the content the chain writes, not the
  on-disk hash. `is_approved(path, write_text)` applies the drift rule
  first (disk no longer hashes to stored -> entry deleted, false), then
  matches the stored hash against the hash of the write. `Chain::
  final_writes(wanted)` returns path + after text of the last
  content-writing step per file; both sides of the approval use it
  (`handle_chain`, `handle_overwrite`, `refresh_trust_after_chain_
  confirm`). Every store function has an `_in` variant taking a folder,
  so tests use a temp dir.
- Step 7: `chmod +x <path>...` verb. `Verb::is_special` (cargo, run,
  chmod) and `Verb::always_reviewed` (those plus zaprm, zapmv, rmdir).
  The 168-point wide ALLOW CARGO/RUN/EXEC button replaces the green
  Confirm on special steps; `confirm_gate` is a pure function taking
  wanted/visited/verified/allowed. Special steps draw a thick frame in
  the banner colour. chmod gets the `EXEC REQUEST` banner, orange
  accent, `cx` glyph. `exec_mode_for` maps 644->755, 640->750,
  600->700, 400->500, 000->100; writer pushes `Undo::RestoreMode`.
  `refresh_trust_after_chain_confirm` builds its surface-refresh list
  from `final_writes(wanted)`, not snapshots, so a chmod cannot re-
  trust an out-of-band edit to a `build.rs`.
- Step 8: single-file whitelisting (a whitelist line pointing at a
  regular file matches only that file; a folder or non-existent path
  matches by components). Hidden paths in ~/.config/zap/hidden.txt,
  matcher in zap_wire/src/hidden.rs. Every read, run and chain refuses
  a hidden path with "deliberately hidden by the user: <path>". The
  menu has a Hidden files section under whitelist.
- Step 10: extension allowlist in ~/.config/zap/extensions.txt,
  matcher in zap_wire/src/extensions.rs. Only create, replace,
  overwrite and append check. zapgui's handle_overwrite (reached by a
  client that bypasses zapcli) calls the same check. A broken file
  fails closed. The menu has an Allowed extensions section under
  Hidden files, using whitelist::{load_extensions, save_extensions,
  validate_extension_entry}. `retry N ...` with replacement bodies now
  resends the whole unapplied payload with step N substituted, so a
  fix to one step and a full retry can be pasted in one go.
- Step 11: hints on failed steps (with_hint, failure_hint,
  replace_hint, describe_closest, whitespace_match, suggest_name,
  missing_file_hint in play.rs; closest_line, edit_distance, tokenize
  in similarity.rs). Parse errors collect: ParseFailure wraps a
  Vec<ParseError>, sorted by line, deduped; Display joins with "; " up
  to 20 then " (+ N more)". parse and parse_with return
  Result<Vec<Step>, ParseFailure>; ParseFailure.first() for tests.
  Verb::from_word holds the word-to-verb match. header_looking,
  skip_past_end and next_header are the recovery helpers.
  report::truncate is pub so play.rs uses one truncation helper.
- Step 12: rename <path> <new-name> verb, dest set at parse time to
  the sibling path. play_one shares Move's arm and calls
  play::path_change_rows so the diff shows the path change without
  inventing content rows. BannerKind::Rename, cyan accent, glyph nm.
  Edit-title pen: pen_ui in glass_button/ui.rs, small button left of
  the full-file pill, shown when Verb::has_editable_target is true
  (create, mkdir, zapmv, rename). Enter commits through
  replan_with_target and replace_pending_if; the old chain's waiter
  gets ChainOutcome::Superseded. The new chain carries an overrides
  list; report::build adds one clause per applied override.
  refresh_trust_after_chain_confirm treats Rename like Move.
  Verb::ALL and a table-driven test pin name round-trip, is_special,
  always_reviewed and has_editable_target. Move's caption
  duplicate-source bug is fixed.

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
- The theme key in the settings file is `theme` (config.rs get_key and
  syntax.rs configured_theme both read it). The plan wrote
  `syntax_theme`. If zap-settings writes `syntax_theme`, one of the two
  is wrong; the theme combo will otherwise do nothing. One-line fix at
  whichever side is wrong.
- If XDG_CONFIG_HOME is set, config lives under $XDG_CONFIG_HOME/zap
  instead of ~/.config/zap. The old settings file is not migrated; a
  fresh theme pick writes a new one in the XDG folder.
- `zap_wire_theme_off()` at the bottom of drawing_requirements.rs is a
  hardcoded `"off"` because zapgui/app doesn't depend on zap_wire. Re-
  export from syntax.rs if that ever grows.
- verb_glyph shares `ed` for replace, overwrite and append (as the plan
  intends). Step 12's "pairwise distinct" glyph test must be loosened
  for those three; the plan's test wording is wrong on this point.
- Syntax table builds that fail (missing syntax, syntect error) print
  `zap: <method> <file>: <message>` and then draw plain. There is no
  logging helper in app_core; the eprintln in syntax.rs's
  configured_theme is the only such path.

## Where the pending decisions live

implementation_plan_2026-10-16.md is the source of truth for the next
batch. Its header is the session contract; steps 1-3 are the format
template; step 4 onward are the settled designs. When a step's design
turns out wrong at implementation time, amend the step in the plan; do
not restate it here.