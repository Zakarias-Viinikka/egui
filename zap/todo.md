# todo — immediate

Details are Claude's call unless stated. Items marked [part 2] or
[part 3] belong to a later split of the current work, not the diff
engine pass that just landed.

## viewer

- Word wrap arrow color is bright orange and stands out. If it turns
  out too loud in practice, tone it down. Tune after living with it.
- The diff viewer adds about one line above the actual edit, so a
  one-line edit usually shows two lines as edited. Annoying. Fix the
  context calculation so a one-line change shows as one line.

## chain internals

- Move-detection thresholds (DICE_FLOOR, ALIGN_FLOOR, LONG_RUN) are
  first guesses. The bug that made moves read as delete+insert is
  fixed now, so these can be re-checked against real cases. Watch for
  false positives or missed moves.

## zapcli — verbs


- [part 2] `overwrite` with content identical to what's already there
  should skip that step and continue the chain, not fail it.
- [part 2] `retry`: re-run only the steps that didn't apply. Works for
  both a failed chain and a chain where the user marked specific steps
  and confirmed the rest. Denials and failures share one record.
  `retry` with no args = everything not applied. `retry 2` = step 2.
- [part 2] When `cat` can't find a file, the failure should also show
  in the CLI terminal, not just the clipboard blob.
- Chain writes still require absolute paths and this stays.
- `cd /path && c c` inside a chain is currently rejected with "cd must
  be followed by cargo or run". A casual `cd` with `&&` shouldn't be a
  chain candidate. Loosen the detection.

## zapcli — bugs

- When zap rejects an edit, it should report every rejection it can
  find in the chain instead of stopping at the first one. Today the AI
  fixes one error, resubmits, and hits the next one that was already
  detectable.
- Word wrap on the input line is wrong. Pasting wraps but continuation
  lines start indented; typing runs past the right edge.
- `last` should give diff plus current contents, and copy to clipboard
  by default.
- `s` opens settings today; it should do what `s` does in the bashrc.
- Add `cdf`, defined in the bashrc.
- [part 3] `edit` command: fzf-pick a file like the `run`/`cdf`
  picker, then open the selected file in Mousepad.
- If a shortcut is pressed and zapgui is already running, the shortcut
  should open zapgui instead of launching a second one.

## zapcli — copy and paste

- [part 3] Select text and right-click should give a copy button.

## zapgui — permission flow

- [part 2] Right-side glass blocks should be colored by the type of
  action or request they represent, so different operations are
  visually distinguishable. Create/edit/move also need clearer
  indicators than they have now.
- [part 2] A dedicated "Allow Special Request" confirmation for
  execution requests (`run`, `cargo`). These must not be approvable by
  the ordinary green confirm. Give the panel a distinct colored border
  or accent.
- [part 2] A verification button for ordinary file reviews, so a file
  can be explicitly marked verified and moved on without special-
  request approval.
- [part 2] Add a `Mark` button alongside Confirm and Deny All. Confirm
  writes unmarked steps and leaves marked ones as the retry payload.
  Mark toggles on the shown step. Deny All writes nothing and makes
  the whole chain the retry payload; marks set before Deny All are
  preserved. Retry is possible only when at least one step is marked,
  or after Deny All.
- [part 2] Double-confirm bug: when a run/cargo request is appended,
  the user confirms it once, then a second popup appears for the same
  request. Make it one confirmation.
- [part 3] Show an indicator of the max edit boundary — something like
  "edits are only happening in /abs/folder/" — so it's visible at a
  glance that nothing above that path is being touched.

## security

- [part 3] Even inside a whitelisted folder, editing `.sh` or `.toml`
  files must require approval. Approval is remembered per path and
  content hash, like run_trust. If content and path are unchanged
  since the last approval, no re-ask.

## feature adds

- `ast-grep` support, and safe Rust renaming.
- Whitelist a single file (not just a folder).
- Mark a file as hidden and never show it. The AI can't touch it or
  see it. If it asks for the file, or tries to cat it, the reply is a
  clipboard line saying the file has been deliberately hidden by the
  user.

## from todo_later

- report::build has no length cap; long chains make a long report.
- popup position and wording are first guesses; tune once popups work.
- rm path with a space: the line splits on whitespace, so `rm a b` is
  two paths. Same class of bug may apply to the new rmdir.
- file extensions should be an explicit allowlist, and every allowed
  format needs a stated behavior per verb.
- a chain that mixes auto-apply-safe steps with one rm/mv should only
  ask for approval on the rm/mv step, not the whole chain.
- a chmod step for chains: `chmod +x <path>`, always reviewed, with a
  distinct "EXEC REQUEST" banner. Human approves before a file becomes
  executable.
- replace failures should be smart. When the old text isn't found,
  locate the nearest matching lines and include them in the clipboard
  sentence ("expected 1 occurrence of `fn foo() {`, found 0. Closest
  match at line 47: `fn foo() -> bool {`."). Reuse text_diff/similarity.rs.
  Same treatment for create-target-exists and overwrite-file-missing
  when cheap.
- the `edit_title` pen button should work: turn the block into a text
  field, Enter commits.
- a rename command.
