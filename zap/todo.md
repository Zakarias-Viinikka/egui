# todo — immediate

Things we've decided. Details are Claude's call.

## diff viewer
(zapgui/every_page_ever/src/ReusableComponents/diff_viewer/ui.rs,
 zapgui/app_core/src/text_diff/)

- Word wrap. Long lines overflow both sides. Rows are fixed height and
  need to grow when a line wraps. Draw-time only; nothing in text_diff
  changes.
- Edit vs replace. Every non-equal pair is RowKind::Change today, so a
  comma added and a line completely swapped out look identical. Add a
  similarity test in pair.rs: similar -> edit, dissimilar -> delete on
  the left, insert on the right.
- Per-side bar color. A Change row draws the same blue bar on both
  sides. Even for a genuine edit one side lost something and the other
  gained it. Color each side separately.
- Move detection. Same content disappearing here and reappearing there
  should read as a move, even with small edits. Fuzzy match plus some
  visual tag.
- Full-file view on click. Clicking a diff should show the whole file
  with the change in context, not just the snippet.
- Select and copy code. Text is drawn with Painter::text today; nothing
  is selectable.
- Deny should turn that step's block in the right-hand column red.

## chain internals
(zapgui/app_core/src/chain/)

- Marker escape. @@ at the start of a content line means one literal @.
  Replaces the current "extra @ on every marker" rule. Shape is Claude's
  call if it sees a better one.
- Failure report should name the step index and verb, so a paste-back
  locates the problem without a follow-up question.
- Run steps in chains. A chain should be able to include a cargo run or
  a file run as a step. Socket has to stay open through review, the
  writes, and the run. Shape is Claude's call.

## popups

- Popups don't show at all. popup.rs calls popup_demo_app, which does
  not exist. The real binary is fading_popup in
  ~/ProgStuff/egui/reusable_components/fading_popup/. It isn't built by
  build_both.sh either. Rename the target, add it to build_both.sh,
  confirm local_cargo.sh vs the RUSTFLAGS the build script already sets.

## zapcli verbs

- cat with multiple files. Same output as the (for f ...) block: one
  clipboard blob, === path === headers.
- append verb. Marker form like create.
- --no-newline flag on create and append. Suppresses the automatic
  trailing newline.
- Chain writes require absolute paths. create/replace/overwrite/append/
  rm/mv/mkdir refuse a bare relative path at parse time in zapcli, with
  a clear error, before anything reaches zapgui. ~/ is fine (resolves
  to absolute). Reads (cat/ls/head/tail/wc) stay relative-friendly.
  Reason: the AI has no reliable model of zapcli's cwd, so a relative
  write is a silent wrong-folder bug or a wasted round-trip.