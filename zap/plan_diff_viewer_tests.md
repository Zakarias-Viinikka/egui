# Plan: diff-viewer regression tests, then incremental workspace split

## Guardrails and order

This work has two separate passes. First make the render-plan contract
testable and make the tests green while the current workspace layout
is unchanged. Only after that gate passes should the new diff crate be
introduced. Do not move files between crates during the test pass. Do
not change DiffRow, RowKind, or Side; do not change move-detection
thresholds as part of this task. Keep the existing egui source patch
and dependency pins untouched, and do not run cargo update.

Before editing, reread the live CONTEXT_FOR_AI.md, NOTES_FOR_NEXT_AI.md,
todo.md, todo_later.md, and CHAIN_REVIEW_UI.md, then inspect git status
and the existing diff. The provided handover identifies in-flight work
on the one-line context calculation and wrap-arrow appearance, and a
review of move-detection thresholds. The handover also mentions a more
detailed move-display change being handled in parallel, but that detail
is not fully present in the supplied todo.md excerpt. Preserve any live
edits and reconcile the actual current state before touching the same
files. The viewer refactor will touch diff_viewer/ui.rs, so
coordinate/rebase rather than overwrite concurrent work there. The move
presentation should be implemented in the new builder; threshold tuning
and general pairing-context changes stay separate.

## 1. Freeze the current baseline and identify the current call path

Read the full current
zapgui/app_core/src/text_diff/{rows.rs,pair.rs,post_process.rs,similarity.rs,moves.rs}
and zapgui/every_page_ever/src/ReusableComponents/diff_viewer/ui.rs,
not just their declarations. Trace where per-step rows and MoveSpans
are produced and where the review and surface-review windows hand them
to the viewer. Include the chain-wide caller, likely in
app_core/src/chain/play.rs, and the current surface-review data/call
sites; confirm exact paths from the live tree. Keep CHAIN_REVIEW_UI.md
as the source for UI behavior that is not part of the row dump.

Files: read-only inspection of the files above, relevant call sites,
both current workspace manifests, and the in-flight diff. No source
files change in this step.

Build/test: record the current successful/failing baseline using the
project's normal test command (c t from the zap root, or the equivalent
explicit Cargo commands). Do not fix unrelated pre-existing failures
under this plan. This baseline is needed to tell a regression from an
existing failure.

## 2. Define the plain-text dump contract and author expected fixtures

Add snapshot-style tests and fixture files in the existing app_core
text-diff area, adjacent to the eventual builder. Each fixture is one
dump of a displayed diff: every output line is one render row
containing both the left and right text, rather than separate left-side
and right-side files. Use exactly seven fields in this order: row kind,
left bar color, right bar color, left text, right text, tag, ghost
flag. Use pipe-separated fields with deterministic escaping for
backslash, pipe, tab, and carriage return; each fixture line must
remain one physical line. Use stable textual names for colors (none,
red, green, violet) and lowercase row-kind names. Empty text/tag fields
represent absent values where the row kind makes that distinction
clear. Do not add a header or include UI-only wrap markers,
continuation markers, line numbers, block separators, or per-character
syntax colors. Keep line numbers in the actual plan if the painter
needs them, but exclude them from this dump format as requested.

Cover ordinary Equal, Change, Delete, and Insert rows; a token-level
change and an unrelated delete-plus-insert replacement; whitespace/
empty-line behavior; multiple separated change blocks; and tags/bars on
the correct side. Add move cases for a same-step move, an edited
same-step move, and a cross-step/cross-file move. Move cases must
construct the entire chain before building the displayed plan, because
the destination cannot be interpreted correctly from a single step in
isolation. For a chain test, the snapshot still represents each
displayed diff as a combined left/right dump; keep the chain context in
the test input rather than inventing a step field in the seven-field
format.

Do not freeze the old context-window bug as expected behavior. Prefer
fixed row inputs or the full-row pairing path for render-plan snapshots
so the todo.md one-line-context fix can land independently. The
changes-only/full-file toggle is not a dump field or a separate
snapshot mode; test the plan for the rows selected by the caller, and
do not serialize UI mode state.

Files: new tests and expected .txt fixture files under
zapgui/app_core/src/text_diff/ (or a nearby testdata directory if that
is clearer after inspection). Keep existing pair.rs and moves.rs unit
tests in place.

Build/test: this test-contract step may be red until the builder
exists. Do not move crates to make it pass. The next steps bring these
tests to green in the current workspace.

## 3. Add a pure render-plan data model and chain-level builder in the current crate

Create app_core/src/text_diff/render_plan.rs and export it from the
current text_diff/mod.rs. Its public data must use ordinary Rust data
only: no egui, eframe, Color32, Rect, Galley, or other UI types. Each
planned row needs its logical RowKind, optional left/right text and
line number, optional left/right bar-color enum, optional tag
information, and a ghost flag. Use a small color enum that maps to the
dump names and to the viewer's existing red/green/violet colors. Keep
left and right tags distinguishable in the data model because the
current painter can draw a tag strip in either pane; if the dump has to
encode both on one row, encode them deterministically with side
prefixes and escaping inside the single tag field.

The builder must take all step inputs together (each step's path, paired
rows or processed blocks, and move metadata) and return render plans
per step. Do not build a single displayed step before examining its
chain peers. This is the boundary that permits move presentation to be
tested without the UI while preserving step-index information. Preserve
current block-boundary information in the data needed by the painter,
but do not add it to the seven-field dump. Preserve line numbers for the
painter's gutters, even though they are intentionally absent from the
dump.

Use the requested move transformation here, not in the egui viewer. At
an origin's old position, the original deletion remains visible as a
dimmed ghost with no bar. For a move within one step, the destination's
insertion run becomes a paired old-text/new-text run, with violet bars
on the populated sides and the move label at the destination. For a
move between steps/files, the origin becomes a ghost, while the
destination keeps ordinary inserted rows and gets the moved from
<path>:<line> tag; it is not converted into a paired same-file move.
Use the step relationship from the full-chain move result to
distinguish these cases.

Do not parse human-readable labels to infer which origin matches which
destination. If the current MoveSpan representation does not preserve a
structured link between the accepted From/To pair, add the smallest
explicit pair identity or companion move-link type at the detector
boundary, and pass that link through to the builder. Keep the existing
DiffRow, RowKind, and Side types unchanged. This may touch moves.rs,
which is under threshold review: inspect the live diff first and
isolate that small API change rather than merging it into threshold
tuning.

For same-step destination rows, reuse the existing line-pairing/
alignment logic over the matched source and destination text, and
restore the absolute source/destination line numbers afterward. Do not
blindly zip lines by index or infer pairing from the rendered label;
spans may be edited or have different lengths. Keep the source ghost at
its original location as well as reusing source text on the
destination's left side. Do not duplicate the (edited) suffix: moves.rs
currently includes it in the generated label while the viewer also
appends it, so normalize the label once in the builder and add a
fixture asserting one visible suffix.

Files: new app_core/src/text_diff/render_plan.rs;
app_core/src/text_diff/mod.rs; focused tests/fixtures from Step 2; and
only the minimal move metadata/call-site changes that the live
implementation requires.

Build/test: run the render-plan snapshot tests and the existing
app_core diff-engine tests. Bring them green in the current workspace
before touching workspace membership or moving files. Also run
formatting and clippy/check if that is part of the current project
routine.

## 4. Turn the egui viewer into a painter of the render plan

Update every_page_ever/src/ReusableComponents/diff_viewer/ui.rs and its
call sites so the viewer consumes render-plan rows rather than deriving
move behavior from DiffRow plus MoveSpan. Remove viewer-side decisions
for bar_color_for, move membership, span-start tags, and ghost
transformation. For each plan row, the viewer may still lay out text
with egui, calculate wrapping and row height, allocate rectangles,
paint the requested bar/tag, draw line numbers, and place selectable
labels. Ghost rows should use a dim text color and must never paint a
bar. Word-wrap markers and selectable-label behavior remain UI
responsibilities and are deliberately not represented in the dump.

Preserve the existing three-pass layout/paint/label behavior, left/right
pane geometry, copy/select behavior, scroll-to-first-change behavior,
right-side accent, and block dividers. If one of these depends on
current processed-block boundaries, carry enough plain metadata through
the plan rather than making the viewer reconstruct diff semantics. Keep
debug_format and the user's existing clipboard-copy text separate from
the test dump; do not silently replace the copy format with the snapshot
format. Keep unrelated wrap-arrow tuning and context-window changes
separate, and reconcile any concurrent edits in ui.rs before applying
this step.

Files: every_page_ever/src/ReusableComponents/diff_viewer/ui.rs, its
module if signatures require it, and the exact review/surface-review
call sites found in Step 1. The render-plan module and fixtures remain
in the existing crate at this point.

Build/test: run render-plan snapshots, cargo test for the zapgui
workspace, and the normal all-project test command. Manually compare a
normal edit, insertion/deletion, a same-step move, and a cross-file
move to ensure that the UI still paints the plan faithfully. This is a
check of the adapter, not a reason to add wrap geometry or syntax spans
to the dump.

## 5. Establish the hard gate before any crate/file move

Do not start the workspace split until the snapshot suite is green, the
viewer consumes the render plan, and every relevant current-workspace
test/build passes. Review the final diff for this pass and verify that
it has not changed DiffRow, RowKind, or Side, the egui patch/version,
cargo lock/dependency pins, unrelated todo items, or move thresholds
except for the narrowly necessary structured-pair link described in
Step 3. Preserve any existing unrelated dirty files. If the detailed
move-display work currently being done in parallel has different
expected semantics from Step 3, stop at this gate and have the human
resolve that behavior before the workspace split.

Files: no new files in this gate; review only the files from Steps 2-4
and the current Git diff.

Build/test: run the project's complete current test command (c t) from
the zap root and require green results. The seven-field snapshots are
the acceptance baseline for the next pass.

## 6. Start the unmonolith pass with one egui-free diff crate only

After Step 5 passes, introduce the first new crate for the pure diff
domain, using the existing zapgui Cargo workspace as the initial
boundary so no whole-repository/workspace-root redesign is required
just to begin. Prefer a clearly named crate under the zapgui/shared/
area (for example, zapgui/shared/diff) if the live zapgui/Cargo.toml
workspace layout supports it; otherwise use the closest clean member
directory under that same workspace and record why. Add one workspace
member and one path dependency; do not restructure zapcli, zap_wire,
and zapgui together as part of this step.

Move the pure diff modules and render-plan builder into that crate: row
types, pairing, post-processing, similarity, move detection/metadata,
dump serialization, and the new render plan. The new crate may depend
on the existing line-diff library and standard Rust facilities, but it
must not depend on egui, eframe, the viewer crate, or app_core. Keep
names and behavior stable. Where callers already use app_core::text_diff,
retain a temporary thin public re-export/facade so this first extraction
does not force a large import rewrite; make the UI consume the plan type
from the pure crate directly where practical. Do not remove eframe from
app_core unless a real dependency/use audit proves it is no longer
needed; the guarantee for this step is that the new diff crate itself
is egui-free.

Files: the new diff crate manifest and source files;
zapgui/Cargo.toml; the minimal app_core/Cargo.toml and
app_core/src/text_diff/ compatibility facade; and only those consumer
manifests/imports needed to use the new crate. Preserve existing
package versions and pinned dependency configuration.

Build/test: run the diff crate's tests independently, then app_core
tests, the every_page_ever compile/tests, the complete zapgui workspace
suite, and finally c t. The same render-plan fixtures must pass
unchanged after the move. Also inspect the new crate's dependency tree
to confirm that no UI dependency entered it.

## 7. Stop after the first crate extraction and use its dependency graph to choose the next seam

Do not attempt a mass split of trust, path handling, chain parsing,
process execution, and UI in the same pass. After the diff crate is
independently green, inspect the remaining dependency graph and select
the next single domain that can be extracted without depending on egui.
zap_wire::chain is already the shared parser used by zapcli and zapgui,
so do not create a competing parser or move that logic merely to match
directory names. Use the server_template_3 structure as a naming/layout
reference for later focused crates, not as a mandate to reproduce all
of its crates at once. Each later extraction should get its own small
plan, move one responsibility, preserve a stable public boundary, run
that crate's tests, then run the dependent workspace tests before
proceeding.

Files: no source changes required for this checkpoint; update this
plan or create a follow-up plan only after examining the real dependency
graph and discussing any scope conflict with the human.

Build/test: the diff crate and full c t suite remain green. Keep the
complete crate list intentionally undecided until the first extracted
boundary provides concrete evidence about the next one.

## Completion criteria

The test pass is complete when the render-plan dump is deterministic,
all planned snapshots pass, the viewer is a painter rather than the
owner of move semantics, and the full existing tests pass before any
crate move. The first unmonolith milestone is complete when the pure
diff crate passes its own tests and the render-plan snapshots still
pass unchanged through the current application consumers. No further
domain should be moved as part of that first crate-extraction step.

---

## Here's the actual implementation plan

Implement the plan above in the zap project. Read the live
CONTEXT_FOR_AI.md, NOTES_FOR_NEXT_AI.md, todo.md, todo_later.md, and
CHAIN_REVIEW_UI.md first, in that order, then inspect git status and
all in-flight diffs before editing. Do not overwrite or discard
concurrent work. The plan explicitly calls out likely overlaps in
diff_viewer/ui.rs, the pairing context behavior, and move detection.
If the actual in-flight move-display specification differs from the
plan's stated behavior, ask the human to resolve that one conflict
before implementing the conflicting part.

Work in order. First add the plain-text render-plan snapshot contract
and its tests in the current workspace layout. Implement the egui-free
render-plan builder and its whole-chain move transformation, make the
tests green, then refactor the viewer to paint the plan. Do not
reorganize crates before the snapshot suite and full current-workspace
tests are green. Same-step moves must leave dimmed, bar-free ghosts at
the old location and show paired old/new text with violet bars and a
move label at the destination. Cross-step/cross-file moves must ghost
the source while keeping ordinary inserted destination rows and adding
the moved from <path>:<line> tag. Do not infer source/target matches by
parsing user-visible labels; add the smallest structured match identity
needed if the current move metadata lacks it. Reuse the existing
line-alignment logic for edited moves, keep DiffRow, RowKind, and Side
unchanged, and keep all egui types out of the render-plan data model.

Only after the viewer snapshot suite and the project's complete c t
tests pass, start the second pass by extracting the pure diff engine/
render-plan code into one new egui-free crate in the existing zapgui
workspace. Keep an API re-export if needed to avoid an unnecessary
all-at-once import rewrite. Run the new crate's tests independently and
then the dependent workspace tests. Stop after this first crate is
green; do not attempt the full workspace split in the same pass.
Preserve dependency pins and the egui patch; never run cargo update.

Follow the project error-handling rules: do not introduce unwrap() or
unwrap_or_default() unless the default is logically correct; propagate
errors with ? where possible and never swallow errors silently. Make
focused changes, run formatting/tests for each step, and report exactly
which tests passed and any existing/in-flight failures without claiming
success for checks that were not run.
