# Prompt for the implementing AI

You are implementing a planned set of changes to zap, a two-binary Rust project at /home/zakke/ProgStuff/egui/zap. zapcli is a terminal-shaped window that is not a shell. zapgui is the review window that shows diffs and asks for confirmation. They talk over a Unix socket. The shared crate is zap_wire. A stronger AI already worked out the design below from the user's requirements. Implement it step by step. Do not redesign. Where a step says "your call", decide yourself without asking.

## How to run the session

Start by giving the user the `tree` command for the zap folder and one dump command for README.md, CHAIN_CHEATSHEET.md, CHAIN_REVIEW_UI.md, CONTEXT_FOR_AI.md and NOTES_FOR_NEXT_AI.md, unless they are already in your context. Then print only the one-line step titles and begin step 1.

Work on one step at a time. Before a step, say which files you need and give one dump command that lists them. Use a `for f in ...; do echo "=== $f ==="; cat "$f"; done` block ending in his `copycat` script, with absolute paths starting `$HOME/ProgStuff/egui/zap/`. Pick the exact file list yourself from the "Files" line of the step. When more is needed, give a new command; never guess file contents. After a step, stop and wait for the user to say "next". He chooses the order; the numbering is only a suggestion.

If you do not know whether he ran the last thing, ask "did you run the last thing" and assume he did not. If a step fails twice, stop and ask; do not try variants. Never run `cargo update`, never change the egui version (eframe is patched through egui-src and depends on the exact source). Tests run with `c t` from the zap root. End a step's last block with `build_both.sh && zap`.

## How the user works

Short answers, every sentence adding information. Prose paragraphs, no bullets unless he asks. One thing at a time: do not mention other problems or coming complications until he reaches them. Plain words, no jargon, no metaphors or analogies, no dumbed-down versions. He understands bottom-up, so state the real mechanism first in plain terms. When he asks "what is X", give only the definition. Take his words literally and do not argue about phrasing. When he says something does not make sense, do not repeat it; try a different shape. Do not end a message with a question about how to calibrate your answer. Do not ask him to paste output after each step; he reports back when ready or when something breaks. Ask fewer questions than feels natural, and when one is needed ask the single question that unblocks everything. Never say "as I mentioned" or "I flagged this".

## How you deliver edits

Prefer a zap chain (replace, overwrite, append, create with markers), one chain per step. Chain paths must be absolute or start with `~/`; `$HOME` is not expanded in chains. Do not put a shell line such as `cd ... && c c` inside a chain; put build commands outside it. When a change does not fit the chain grammar, give a terminal command. One command per file, never several files in one paste. Targeted edit shape: `cd "$HOME/ProgStuff/egui/zap" && python3 - << 'PY'` reading the file, asserting the exact old text exists, `replace(old, new, 1)` with enough context to be unique, writing back, then `PY`. Whole-file rewrite shape, only when most of the file changes: `cd "$HOME/..." && cat > "$HOME/.../file" << 'EOF'` then the content then `EOF`. Never sed, never perl -i, never nest heredocs, never put `&&` directly after a heredoc delimiter. Paths are absolute from `$HOME`, never copied verbatim from tree output.

## Error handling (hard rules, new code only)

Never use unwrap() or unwrap_or_default() unless the default is logically correct at that spot. Return errors with `?` when the signature allows. Zap's convention is String errors that name the operation and the path (for example "read /x/y: permission denied"); keep it. When a function cannot return a Result (egui draw code, thread bodies), surface the error through a channel that already exists: the window's error_line, or the clipboard report plus popup. Never discard an error with `let _ =` unless it is best-effort cleanup and a one-line comment says so. Run `rg unwrap_or_bail` in the zap folder once; if the macro exists use it where a function cannot return a Result, if not follow the rule above and do not ask about it. Existing code that breaks these rules stays as it is unless a step touches it.

## Already built: verify only, do not rebuild

The user's todo.md and todo_later.md are stale. These exist in the code: overwrite with identical text is skipped (no_op); retry with step numbers and replacement bodies; cat prints a missing-file error line in the terminal as well as the clipboard; Verify, Mark, Allow & Run and Deny All buttons; verb-tinted step blocks; `.sh`/`.toml` approval store (flawed, see step 6); the double-confirm fix; a second manual zapgui launch hands off to the running one; `last` gives diff plus current contents and copies; report length cap; rm and mv were renamed zaprm and zapmv; zaprm, zapmv, mkdir, rmdir accept quoted paths with spaces; popup position follows the display size; edit-vs-replace pairing by token similarity. Step 18 prunes the todo files.

## Not in scope unless he asks

Tuning the word-wrap arrow colour; tail or wc to the clipboard; find, grep, git verbs; `t`, `cur`, `b`, `co`, `cop` commands. The `s` and `cdf` commands are in scope (step 15).

---

## Step 1: Review window polish

Files: zapgui/app/src/egui_setup/drawing_requirements.rs, zapgui/every_page_ever/src/ReusableComponents/glass_button/ui.rs and mod.rs, close_button/ui.rs, README.md, CHAIN_REVIEW_UI.md.

Click guard: set CLICK_GUARD to 150 ms. It already guards Confirm and Deny All through click_ok. draw_surface_review has no guard at all although the README promises one; add a surface_shown_at Instant, reset it where surface_changed is handled, and gate that window's Confirm and Deny the same way. Fix the README wording.

Tooltips: the glass button and pill helpers return only a bool. Make them take a tip string, or return the Response, and attach on_hover_text. Set style.interaction.tooltip_delay to about 0.15 s once at startup. Use these texts exactly. Verify: "Tick this once you have read the step. Run and cargo steps cannot be confirmed until verified." Mark: "Leave this step out. Confirm skips marked steps and keeps them for retry." Confirm: "Write every step that is not marked." Allow & Run: "Write the steps, then run the command in this step." Deny All: "Write nothing. All steps stay available for retry." Close: "Hide this window. The waiting chain stays waiting." Pill: "Switch between the changed lines and the whole file." Step block: "Step N of M: <verb> <file name>", plus ", marked" or ", verified" when set.

Boundary indicator: remove the second caption line. Draw the boundary as one chip on the caption row, right-aligned and just left of the full-file pill, so it costs no height. Text: "confined to <folder>/". If it does not fit, shorten from the left with an ellipsis so the last folders stay visible. Chip max width is 45% of the caption area; the caption wrap width shrinks by the chip width plus a gap. Colours: green border and text normally; amber when the folder has two or fewer path components (for example /home/zakke); red-orange with the text "edits span the whole disk" when it is "/". layout_caption_with_boundary becomes a plain caption layout plus a separate chip layout.

Done when a one-file chain shows the chip on the caption row and hovering each button shows its tip.

## Step 2: Diff pairing and viewer fixes

Files: zapgui/app_core/src/text_diff/pair.rs, post_process.rs, rows.rs (read only), zapgui/every_page_ever/src/ReusableComponents/diff_viewer/ui.rs.

Cause of the stacked one-line change: the diff library already returns Replace ops, pair_one_op collects the old and new lines, and align() pairs only lines whose token similarity is at least 0.5. A short line replaced by a longer one scores far below that, so it comes out as a Delete row then an Insert row. Fix in emit_aligned: when an unmatched gap (the run between matched pairs, or the whole op) holds exactly one old line and exactly one new line, emit one Change row (Equal if same_text). Gaps with more lines keep today's behaviour (deletes, then inserts). DiffRow, RowKind and Side do not change. Tests in pair.rs: change unrelated_swap_gives_delete_then_insert to expect [Change]; add "let a = 1;" to "let a = compute_something_long(b);" gives [Change]; "x = 1;" to "y = 2;" gives [Change]; two unrelated old lines to two unrelated new lines gives Delete, Delete, Insert, Insert; identical text on both sides gives Equal.

Cause of the extra line above an edit: post_process::process pops the last Equal row into the changed block, and the viewer draws its horizontal rule above that block's first row, so the context line looks edited. Remove the pop: a changed block holds only changed rows, and Equal rows stay in their own blocks. Test: three Equal rows, one Change row, three Equal rows give blocks of 3, 1 and 3 rows.

Viewer, ui.rs. (a) The rule and first_change_rect are gated on block kind == Change; gate on kind != Equal so insert-only and delete-only blocks also get a rule and the initial scroll. (b) Pass 0 appends " (edited)" to labels that moves.rs already ends with " (edited)"; remove the viewer's append. (c) When both tags exist on one row, y advances twice but the height was added once; paint both strips at the same y and advance once. (d) Selection fix: labels are placed per row, left then right, so egui's in-between selection logic selects the other pane's rows while dragging inside one pane. In the row loop only allocate, paint bars, numbers and strips, and record each row's y and galleys. After the loop place all left labels, then all right labels.

Done when a one-line edit shows as one row with the context lines clearly above it, and dragging a selection inside one pane never highlights the other pane. If the selection still leaks, stop and ask.

## Step 3: Move display

Files: text_diff/moves.rs, pair.rs, mod.rs, text_diff/ (new move_display.rs), chain/play.rs, diff_viewer/ui.rs, zapgui drawing_requirements.rs.

Spec from the user. Same-file move: at the old position show the moved text dimmed gray, no bar, no label. At the new position show each moved line paired side by side with its own old copy, one row like a Change, with violet bars on both sides and the violet label strip above the first row. The old copy shown on the left may be a lie about where it lives. Cross-file move: the old spot in file A shows the same dim copy; in file B the inserted rows keep violet bars and the strip "moved from <path>:<line>". Full-file view stays exactly as today. DiffRow, RowKind and Side do not change.

Design. (1) MoveSpan gets three fields: pair (index of the confirmed match, unique across the chain), partner_step, partner_first_line. Set them in detect() where the spans are created. Do not parse labels. (2) pair.rs exposes pair_runs(old: &[Side], new: &[Side]) -> Vec<DiffRow>, a thin wrapper over align plus emit_aligned. (3) New text_diff/move_display.rs with apply(rows: Vec<DiffRow>, spans: &[MoveSpan]) -> Vec<DiffRow>, run on the flat pair_lines rows of one step before process(). For each To span whose partner_step is this step: collect the old Sides from the Delete rows whose left lineno lies inside the paired From span; find the contiguous Insert-only rows whose right lineno lies inside the To span; replace that run with pair_runs(old_sides, new_sides); then set every produced row whose right lineno is in the To span to kind Change, even when the texts are equal, so a pure move still reads as a change and still gets a rule and scroll. The left Sides keep their original old-file line numbers on purpose. If either run is not found, leave the rows untouched. Cross-step pairs need no row change. (4) play.rs: after move_spans are stored in play_forward, for each step that has spans, recompute rows = process(&apply(pair_lines(before, after), &spans)). (5) Viewer: DiffViewerParams gets a field moves: MoveDisplay with Classic and Dimmed. In Dimmed (snippet view): a Delete row whose left lineno is inside a From span draws its text in one dim gray (new constant), with no bar and no strip; From tags are never drawn; rows in To spans get violet bars and the To strip; a Change row with left in From and right in To gets violet bars on both sides. Classic (today's behaviour) is used when full_file is on and for surface reviews. drawing_requirements.rs passes Dimmed unless full_file is on.

Tests: a same-file move of five lines gives five Change rows plus five dim-eligible Delete rows; an edited move with one changed line gives Change rows with different texts; cross-file spans leave rows unchanged; two realistic Rust cases (a function moved inside a file is detected; three repeated closing-brace lines are not detected as a move). A false positive now hides a real deletion, so do not lower the thresholds in moves.rs.
