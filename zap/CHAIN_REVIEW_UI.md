# CHAIN REVIEW UI

The chain review window is drawn by
`zapgui/app/src/egui_setup/drawing_requirements.rs`. It reads the
pending `Arc<Chain>` from `terminal_router` every frame.

## Layout

`PageLayout::new` computes the rects once per frame: banner (optional),
caption (height grows to fit), viewer, block column (right), button row,
close button, error line, pill. All in points.

A surface review uses the same layout with a different caption and
banner.

## Banner

Delete, rename, execute and cargo-run each have a banner at the top with
their own colors and hint. `BannerKind` in `surface_review.rs`. The
banner takes its height off the top of the caption, so the rest of the
layout shifts down when one is present.

## Caption

One galley, monospace 13. Reads `step n of m · verb path`. Suffixes:
` · marked for retry`, ` · verified`. The caption is laid out with
`layout_caption_with_boundary`, which appends a second, dimmer line
naming the chain's `boundary` (deepest common folder of every path the
chain touches). A colored accent bar sits at the caption's left, in the
shown step's verb color.

## Viewer

`every_page_ever::ReusableComponents::diff_viewer::ui`. Three-pass
renderer. Wrap and per-side bars and move strips are documented in the
top of that file.

## Buttons

Four glass buttons in a row, centered on the viewer:

- **Verify** (steel circle with a check). Toggles the per-step
  `verified` flag. On a run or cargo step, Confirm refuses to fire until
  this is set.
- **Mark** (amber bookmark). Toggles the per-step `marked` flag. A
  marked step is skipped by Confirm and put into the retry payload.
- **Allow & Run** (purple play triangle) on a run or cargo step, or
  **Confirm** (green check) otherwise. Writes every unmarked step.
- **Deny All** (red X). Writes nothing, makes the whole chain the retry
  payload, preserves any Marks set before.

Mark and Verify are toggle buttons: no click guard, and the shown diff
recenters on the click. Confirm and Deny All are guarded by the 500 ms
`CLICK_GUARD`.

## Mis-click guard

Confirm refuses to write a run or cargo step whose `verified` flag is
unset, or a `zaprm` / `zapmv` / `rmdir` step the user hasn't looked at.
The window jumps to that step and shows the reason in the error line.

## Block column

One glass block per step on the right. Border tinted by verb
(`verb_accent` in drawing_requirements.rs: create/mkdir green, edit
blue, delete red, move cyan, cargo amber, run purple). State wins over
accent: marked amber, denied red, selected brighter. Clicking a block
jumps to that step.

## Where state lives

Per-frame UI state is in `NecessaryStructForEgui`: `shown`,
`shown_step`, `marked: Vec<bool>`, `visited: Vec<bool>`,
`verified: Vec<bool>`, `full_file`, `scroll_to_change_pending`. All
reset when a new chain is shown.

The pending chain itself lives in `terminal_router::PENDING_CHAIN`, an
`OnceLock<Mutex<Option<Arc<Chain>>>>`.
