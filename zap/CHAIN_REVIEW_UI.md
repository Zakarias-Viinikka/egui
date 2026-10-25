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

`confirm_gate` in drawing_requirements.rs refuses to write a cargo,
run, chmod or rename step whose verified or allowed flag is unset, or
a zaprm / zapmv / rmdir step the user hasn't visited. Jumps to that
step with an error line.

## Edit-title pen

A small 22 px pen button sits just left of the full-file pill when
the shown step's verb has_editable_target (create, mkdir, zapmv,
rename). Clicking it opens a single-line TextEdit over the first
caption line, filled with `target_draft` from play.rs (the resolved
full path, or the bare new name for a rename). While the field is
open, confirm, mark, verify and deny ignore clicks and the error line
reads "Enter applies the new name, Esc cancels".

Enter commits through `replan_with_target`, which clones the applied
steps, sets the edited one's target, remaps later steps whose path,
dest, run-file or (for a mkdir) cwd sits under the old target, and
plays the chain forward. The new chain replaces the old one in the
pending slot via `replace_pending_if`; the old chain's waiter receives
`ChainOutcome::Superseded`. Visited and verified are cleared for the
edited step and every remapped step. Escape, a second click on the
pen, loss of focus without Enter, a step change, a new chain or
hiding the window cancels. `PILL_RESERVE` grew by the pen's width plus
a gap.

## Block column

One block per step on the right, border tinted by verb. Marked amber,
denied red, selected brighter. Click jumps to that step. Each block
shows the step's two-letter verb glyph in the accent colour inside it:
cr create, ed replace / overwrite / append, dl delete, mv move, mk
mkdir, rd rmdir, cg cargo, rn run, cx chmod, nm rename. Surface
reviews draw no glyph.

## State

Per-frame in NecessaryStructForEgui: shown, shown_step, marked,
visited, verified, allowed, full_file, scroll_to_change_pending,
revealed_all, title_edit. All reset on new chain. `review_indices` on
the Chain is the shown subset; `revealed_all` forces every step after
a hidden step failed at confirm. `title_edit` is (step_index, draft)
while the pen field is open. Pending chain itself in
terminal_router::PENDING_CHAIN; swapped by `replace_pending_if` only
when the slot still holds the old Arc.