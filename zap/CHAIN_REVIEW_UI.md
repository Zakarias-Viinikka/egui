# CHAIN REVIEW UI

Last updated: 2026-10-25

Drawn by zapgui/app/src/egui_setup/drawing_requirements.rs. Reads the
pending Arc<Chain> from terminal_router every frame.

## Layout

PageLayout::new computes rects each frame: banner (optional), caption
(grows to fit), viewer, block column, button row, close button, error
line, pill. All in points. Surface review uses the same layout with a
different caption and banner.

## Banner

Delete, rename, execute and cargo-run each have a banner at the top with
their own colours and hint. BannerKind in surface_review.rs. Takes its
height off the top of the caption.

## Caption

Monospace 13. Reads `step n of m - verb path`. Suffixes: ` - marked for
retry`, ` - verified`. A boundary chip on the same row, right-aligned
left of the full-file pill: green normally, amber at two components or
fewer, red-orange `edits span the whole disk` for `/`, ellipsis-truncated
above 45% of the caption area. Accent bar at the caption's left in the
shown step's verb colour.

## Viewer

every_page_ever::ReusableComponents::diff_viewer::ui. Three-pass.
Snippet view uses Dimmed move display (source rows dim grey, no bar or
strip); full-file and surface reviews use Classic.

## Buttons

Four glass buttons centred on the viewer:

- Verify (steel circle). Toggles the per-step verified flag. Required
  for run / cargo steps.
- Mark (amber bookmark). Toggles the marked flag. Skipped by Confirm,
  kept for retry.
- Allow & Run (purple play) on run / cargo steps, else Confirm (green
  check). Writes every unmarked step.
- Deny All (red X). Writes nothing, makes the whole chain the retry
  payload, keeps Marks.

Verify and Mark have no click guard. Confirm and Deny All guarded by the
150 ms CLICK_GUARD in both windows. Every button, the pill, the close
button and the step blocks have tooltips; delay 0.15 s.

## Mis-click guard

Confirm refuses to write a run or cargo step whose verified flag is
unset, or a zaprm / zapmv / rmdir step the user hasn't visited. Jumps to
that step with an error line.

## Block column

One block per step on the right, border tinted by verb. Marked amber,
denied red, selected brighter. Click jumps to that step. Each block
shows the step's two-letter verb glyph in the accent colour inside it:
cr create, ed replace / overwrite / append, dl delete, mv move, mk
mkdir, rd rmdir, cg cargo, rn run, cx chmod. Surface reviews draw no
glyph.

## State

Per-frame in NecessaryStructForEgui: shown, shown_step, marked,
visited, verified, full_file, scroll_to_change_pending. All reset on new
chain. Pending chain itself in terminal_router::PENDING_CHAIN.