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

Four controls in a fixed row centred on the viewer; positions don't
change when the shown step changes:

- Verify (steel circle). Toggles the per-step verified flag. Required
  for cargo, run, chmod and rename steps.
- Mark (amber bookmark). Toggles the marked flag. Mark means "skip this
  step when I confirm"; it stays as the retry payload. Confirm never
  writes a marked step.
- Approve slot (168 points wide). Green Confirm check on ordinary steps
  (writes every unmarked step). On cargo, run, chmod and rename steps
  it holds a wide two-line button (small SPECIAL REQUEST above ALLOW
  CARGO / ALLOW RUN / ALLOW EXEC / ALLOW) with a 3-point accent border
  in the verb's colour. Clicking it sets the shown step's `allowed`
  flag and then runs the same gate and write path as Confirm.
- Deny All (red X). Writes nothing, makes the whole chain the retry
  payload with earlier Marks kept.

`confirm_gate` is the pure gate behind Confirm and the wide button. It
stops on the first wanted step that fails its gate: cargo, run, chmod
and rename need verified AND allowed; zaprm, zapmv and rmdir need
visited. Marked steps are not gated. A stopped press jumps to that step
with an error line naming the missing flag.

Verify and Mark have no click guard. Confirm and Deny All use
CLICK_GUARD (150 ms). The wide Allow button uses SPECIAL_CLICK_GUARD
(600 ms) so a miscued click after a step change can't approve a special
request by accident. Every button, the pill, the close button and the
step blocks have tooltips; delay 0.15 s.

## Right-click menu

Right-click inside either pane box opens a small popup: Copy line, Copy
left/right side, Copy both sides as text, and a hint about Ctrl+C.
Greyed when the clicked side is empty. Closes on pick, Esc or a click
outside. Copying calls app_core::clipboard::copy_to_clipboard then
chain::popup::notify_copied, so a popup reading e.g. "Copied line ·
copied" appears.

## Mis-click guard

`confirm_gate` in drawing_requirements.rs refuses to write a cargo,
run, chmod or rename step whose verified or allowed flag is unset, or
a zaprm / zapmv / rmdir step the user hasn't visited. Jumps to that
step with an error line.

## Popups

Every Confirm, Deny All, failure and whitelist auto-apply calls
`chain::popup::notify_copied(headline, text)`, which writes the
clipboard first and then shows the fading_popup window. The popup is
centred on the monitor in use, sits at the top of the work area so a
review can stay open underneath, lasts 2.0 s total (1.5 hold + 0.5
fade), and replaces the previous popup rather than stacking. Text ends
with " · copied" or " · copy failed".

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