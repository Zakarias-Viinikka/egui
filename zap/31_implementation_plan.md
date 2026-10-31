Everything is solved below. Where your draft’s decision didn’t hold up against the code, I changed it:

delete on folders: The parser needs no change. It never touches the disk, and the “is a directory” refusal lives in play_one. A folder is set aside and purged only after the whole chain succeeds, so a failed chain still restores it. The .sh/.toml question disappears because delete is always reviewed and never executes anything. A hidden entry inside the folder does block the delete.
Duplicate-paste prompt: It continues with y + Enter, like the repair prompt. A double-Enter reflex would defeat an Enter-to-continue prompt, and Esc needs zapcli UI code I couldn’t see. It’s a one-line swap if you want Enter instead.
rg: It runs in zapgui, not zapcli, because zapcli never executes anything. Hidden paths are enforced by negated globs, and flags that run programs are denied.
Full-file pill: It now shows two independently scrolling panes with no gaps. I dropped the scroll-anchor requirement and kept the existing scroll-to-first-change on toggle.
Whitelist menu: It closes on Esc and on a click outside the panel, since your todo said “clicking”.
chmod banner: I renamed it, because “EXEC REQUEST” next to “EXECUTE REQUEST” was the confusion you described.
Retry guidance: It goes through one central function. play_forward failures never reach report::build, so patching only report.rs would miss the case you keep hitting.

Everything below the line is the prompt.

---

You are working on zap, a Rust project at /home/zakke/ProgStuff/egui/zap. Read CONTEXT_FOR_AI.md in the order it lists. Read the top of NOTES_FOR_NEXT_AI.md for how the user works: short answers, prose, one paste per task, edits as zap chains or Python heredocs with absolute paths, and command blocks that end with build_both.sh && zap. Read todo.md; you tick items off at the end. Skip the Zed-refresh item.

This batch is eleven items plus a docs sweep. The designs below are settled, so don’t re-ask. If the code contradicts a design, adapt it, say so in your final note and keep going. Stop and ask only at the two places marked STOP.

Work in this order, running c t from the zap root after each step: 3, 5, 8, 1, 9, 2, 7, 6, 4, 11, 10, 12. Step 6 uses a function from step 7. Several steps touch the same files (report.rs, play.rs, drawing_requirements.rs, route_requests.rs), so finish one before starting the next.

Step 1. ls copies to the clipboard (small).
Return Outcome::PeekAndCopy from ls.rs, as head.rs and tail.rs do. Keep each line’s own LineStyle so folders stay folder-coloured. Set skip_copy = true on every line so a later copy of the scrollback doesn’t repeat the listing. Check how the UI handles PeekAndCopy (it marks the prompt line skip_copy for peeks) and make ls behave the same.

The clipboard text is a first line === <absolute listed folder> === (the same header style as cat), then one name per line in the same sorted order, folders with a trailing slash. Cap the clipboard text at 1000 entries and end with (+ N more not copied) when cut; the scrollback stays uncapped. Hidden refusals and read errors stay plain error prints with no copy. Put the line and text building in a pure helper and test ordering, header and cap.

Step 2. Full-file pill becomes two desynced, gap-free panes (medium).
This replaces the existing full-file view; there’s no new pill, and the pill labels stay. It applies to chain reviews and surface reviews alike, since both toggle full_file.

The idea is that each pane shows its own whole file in its own flow. The left pane is the old file, with red and blue bars on deleted and changed-old lines. The right pane is the new file, with green and blue bars on inserted and changed-new lines. Neither pane has blank holes, they are no longer row-aligned, and they scroll independently.

Add a pure function split_panes(blocks) -> (left_blocks, right_blocks) in text_diff next to process. For each block it keeps the rows that have a left side, clones them with right = None, and keeps kind, edit, line numbers and changed ranges untouched. It does the mirror image for the right side. Blocks left with no rows on a side are dropped from that side’s list. Don’t add fields to DiffRow or Side; any new field needs a decision in every consumer.

In diff_viewer/ui.rs, add desync: bool to DiffViewerParams (default false) and derive Clone and Copy on it (every field is Copy). The body function already supports a single-pane mode (single, which skips the other side’s paint, HRs, wrap marks and hit test). Generalize it with a forced side: when diff_viewer_body is called with a forced side, single is that side and single_rect is the rect it was given, not a centred half.

When desync is on and both sides have content, the public entry points compute the split and call the body twice. The left call gets the left half of the viewer rect (min to mid_x − GAP/2) and the right call the right half (mid_x + GAP/2 to max). Each call gets a distinct scroll_id (derive from the original plus “left”/“right”), which also keeps the right-click menu state separate. Only the right call paints right_accent. Never show the copy button in desync. If either side is empty, skip the split and use the current behaviour, so the centred single pane for created or deleted files stays.

Set desync: self.full_file at both DiffViewerParams construction sites in drawing_requirements.rs. Scroll-to-first-change on toggle stays as it is, per pane. One known gap: the right pane doesn’t mark the points where lines were deleted. The left pane carries those.

Tests for split_panes: a delete-heavy, an insert-heavy and a mixed block list. Check that counts match, each side keeps its own line numbers and no right text appears on the left. Also cover the empty-side case.

Step 3. zapmv says “move” (small).
The BannerKind enum in app_core’s surface_review.rs gets a Move variant. In drawing_requirements.rs, banner_kind maps Verb::Move to it, while Rename and AstGrep keep BannerKind::Rename. Move gets the title “MOVE REQUEST”, the same cyan colours as Rename, and the hint “confirm = move it, deny = don’t”. Fix every exhaustive match the compiler flags. Change verb_word(Verb::Move) (used in block tooltips) from “zapmv” to “move”. Don’t touch the chain verb word. Add a banner_title test for Move.

Step 4. delete takes a folder (medium).
No parser change. Nothing else in zap_wire changes either, so both sides keep the same rule. The work is in play.rs, writer.rs, report.rs and a new chain/dirwalk.rs.

dirwalk.rs (pure, tested).

Walk with read_dir and DirEntry::file_type, never following symlinks. Count files (symlinks and specials count as files), folders beneath the root, and bytes. Also keep per-top-level-child subtotals.
Refuse the whole walk with a one-line reason in these cases: any entry the HiddenSet hides (<folder> contains <path>, which the user deliberately hid, so it can't be deleted); any entry on a different device than the root (compare MetadataExt::dev; that is a mount point); any read error; more than 500,000 entries.
A DirSummary struct holds the counts and the top-level list. A render function builds the review text: a header delete folder <abs path>, a line N files, M folders, X MB, a line no undo copy is kept; the folder is set aside and removed once the chain succeeds, a blank line, then up to 40 top-level children as name/ N files size (a file child shows its own size), then (+ K more entries) if cut. It never enumerates every file. Write a small human_bytes.

play.rs.

AppliedStep gets a new field dir: Option<DirSummary>, and every literal (play_one arms, test helpers) must be updated.
In the Remove arm, use symlink_metadata. A real directory takes the new path. A symlink to a folder is refused with a plain message. Also refuse /, $HOME, and any ancestor of $HOME.
A real directory skips load_once and the 1 MB rule. before is the rendered summary, after is empty, and rows = process(&pair_lines(&summary, "")), which makes a single red left pane.
Update the existing rm_refuses_directories test, since a folder now plans successfully.
Extend exclusivity_failures. For each folder-delete step, any other step in the chain that touches a path under the folder (its path, its dest, or for an astgrep step only its changed files) fails with the same style of message: “step N (...) touches X, which step M deletes, put it in a separate paste”. An astgrep step’s own folder does not count, because astgrep skips target.

writer.rs.

A folder delete in execute_step is a set-aside. Check that the path is still a real directory, rename it to a unique sibling .<name>.zap-trash-<pid>-<nanos>, and push a new Undo::RestoreFolder { trash, original }. undo_all renames it back, so a chain that fails later restores the folder. This keeps the all-or-nothing rule without copying anything.
After the whole apply succeeds, collect the trash paths from the undo list and purge them with remove_dir_all on a detached thread, so a multi-GB target/ doesn’t freeze the UI. Under cfg(test) purge inline. A purge failure goes to eprintln and nothing else.
Don’t re-walk the folder at write time. Build dirs change constantly, and re-walking would make deleting target/ fail while cargo runs. The counts shown are as of paste time.
Skip record_last_change for folder deletes.

report.rs. Describe a folder delete as deleted folder <path> (N files, M folders, X MB), with the usual “would have” prefix.

drawing_requirements.rs.

Add a caption tail · folder: N files, M folders after the astgrep_tail pattern.
The trust cleanup in refresh_trust_after_chain_confirm must, for a folder-delete step, remove everything under the prefix from run_trust, file_trust approvals and the surface trust record. Add _under(prefix) siblings next to remove_all_at_path, remove_approval and trust::remove_from_record. If a store makes this awkward, STOP, skip that store, and say which one in your final note.
The sensitive-file rule needs no change. Delete is always_reviewed, so it never auto-applies, and deleting a .sh or .toml doesn’t execute it.

Tests: dirwalk counts, hidden-inside refusal, symlink not followed, home refusal; play plans a folder delete as one step; exclusivity; writer removes the folder, restores it when a later step fails, and refuses if the path became a file.

Step 5. Tone down the special-request visuals (trivial).

paint_right_accent in diff_viewer/ui.rs draws three glow rings plus a 2 px stroke. Delete the rings and keep one 1.5 px inside stroke in the accent colour.
In drawing_requirements.rs, remove the 6 pt screen-edge frame drawn for special verbs.
In glass_button/ui.rs, read wide_allow_button_ui and strip any glow, shadow, gradient or pulse. Use a flat fill, a 1.5 px border and text in the accent colour, and make hover brighten the border only.
Every verb keeps its accent and banner colour.
Fix the chmod confusion from todo.md: the Exec banner title becomes “CHMOD REQUEST” and the wide button’s big text “ALLOW EXEC” becomes “ALLOW CHMOD”. The hint text stays. Update banner_title_gives_exec_for_chmod.

Step 6. Retry honours the whitelist auto-apply (small).
Extract the tail of handle_chain (from whitelist::load() on) into submit_chain_with(chain, origin, folders, is_unapproved) -> String, plus a thin submit_chain that supplies the real whitelist and the file_trust closure. origin is Paste or Retry { drop_note }. The logic:

If steps_that_need_eyes returns empty, auto-apply with writer::apply.
On success, build the report, show a popup (“Change was made”, or “Retry applied” for a retry), and put the report on the clipboard.
For a retry origin only, also save a retry record with every step Applied, so a later bare retry doesn’t replay applied steps. A Paste origin doesn’t save a record, which keeps today’s behaviour.
A retry returns zapgui: retry applied, <drop_note> when there is a drop note, else an empty string.
On writer failure, do what handle_chain does today: save a Failed record from the chain, show the popup, copy the sentence, and return it.
Otherwise set review_indices when only some steps need eyes, and stash exactly as handle_chain does now, appending the drop note to the replaced-chain sentence for retries.

handle_chain now calls this. handle_retry calls it instead of stash_pending_chain. Leave handle_overwrite alone. Retry’s payload semantics and its hidden-path refusals stay as they are. Always-reviewed verbs still go to review, because steps_that_need_eyes already marks them.

Tests go through submit_chain_with with an injected whitelist: a retry inside a whitelisted temp folder applies immediately and updates the files; a retry containing a delete lands in the pending slot. The pending slot is global, so copy whatever the existing tests do about that, or take a static test mutex.

Step 7. Failure reports teach retry (medium).
play_forward failures never reach report::build, so centralize this in report.rs.

Add failure_sentence(reason) -> String, which returns zap: nothing was written. <reason>. plus the guidance. Also add retry_guidance(reason) -> String, which extracts the failed step numbers from the reason text.

Extraction scans for the exact clause shape step N of M ( and dedupes. Write it as a pure, tested function. Test it on a multi-failure play_forward string, a writer string, and a string with hint text.

If no step numbers are found, add no guidance. The exclusivity failure says “put it in a separate paste”, and retry advice would contradict it.

With five or fewer failed steps, name them. Example: “Nothing from this chain was applied. Do not resend the chain: retry re-runs all of it, taking the original text for every step except the ones you replace. To fix steps 3 and 7, send retry 3 7 on its own line, then exactly one corrected step block for each, in that order. To leave a step out, send retry -3.” With more than five, drop the list and keep the general form. The two facts that must always appear are that nothing was applied and that retry re-runs the whole chain, with the numbers only choosing replacements or drops.

Wire it in:

report::build appends the guidance when failure is Some. Subtract its length from the step budget, next to TAIL_RESERVE, so the cap still holds.
Use failure_sentence at the three sites that save a retry record: the play_forward failure in handle_chain, the one in handle_chain_run, and the writer failure inside submit_chain_with.
In try_confirm (drawing_requirements.rs), the clipboard gets the full report, but the red error_line gets only the plain header sentence, since the line is 30 px tall.
Do not add guidance to parse errors, handle_overwrite or retry_failure. Their numbering or record state doesn’t match.

Step 8. Escape and click-outside close the whitelist menu (trivial).
In draw_menu, set menu_open = false on Escape, and on a primary click whose position is outside the panel rect (the rect expanded by 24 as drawn). The existing hide-on-close logic handles the rest.

Step 9. Duplicate chain prompt (medium).
Session (zapcli command_router/mod.rs) gets last_sent: Option<(String, Instant)> and pending_duplicate: Option<String>. Update the struct literals in the tests.

In core_commands/chain.rs, after a successful parse and before forwarding, compare input.trim() with last_sent. If it’s equal and at least 120 bytes, don’t forward. Set pending_duplicate and print a cli line: “zap: you already pasted this chain <age> ago (<n> bytes). Type y to send it again, anything else drops it.”

Remember a chain as sent only when zapgui accepted it. For a normal chain that means the reply is Ok and doesn’t contain “nothing was written”. For a run chain it means the stream session opened. A chain that failed isn’t a duplicate worth warning about.

The y handler in route currently calls chain::run for the repair case. Split run into run (checks and updates the duplicate state) and run_forced (skips the duplicate check), and have both y paths call run_forced. Otherwise y would re-trigger the prompt forever. y consumes pending_duplicate the same way it consumes pending_chain_repair. Any other non-empty input clears both.

Reads, retry and everything that isn’t a chain are untouched. Tests: first send passes, an identical resend under 120 bytes passes, an identical resend over 120 prompts, y sends without re-prompting, other input clears the prompt, and a failed chain is not remembered.

Step 10. rg verb (medium).
rg must run in zapgui, not zapcli, because zapcli never executes anything. It mirrors how last forwards a request and prints the reply. Study last.rs and copy its reply handling, except that the result is Outcome::PeekAndCopy so the output lands on the clipboard and is skipped by scrollback copy.

zap_wire/src/rg_gate.rs (new, shared like cargo_gate): check(args), operand_refusal(args, cwd, hidden) and build_argv(args, cwd, hidden).
Denied flags. --pre, --pre-glob and --hostname-bin (they execute programs), -L/--follow (a symlink could lead into a hidden folder), and --debug/--trace (they print walked paths). The short-cluster check must be aware of value-taking letters, so -eL (pattern “L”) passes while -nL is denied. Table-test this.
Operand screening. Every token that doesn’t start with - (and every token after --) is resolved against the cwd, with ~/ expanded. If it names a path that exists and is hidden, refuse with the normal hidden message. Also check its canonical form.
Hidden entries inside the roots. The final argv is --no-config, then the user’s args up to any --, then one -g '!<pattern>' per hidden entry, then the rest. Appending the negated globs last matters, because later globs beat earlier ones, including the user’s own positive -g. Escape glob metacharacters in the patterns.
A bare-name entry becomes !NAME.
An absolute entry P (with ~/ expanded) becomes !**/<P without the leading slash>, and also !/<relative path> when P is under the cwd. rg normalizes paths under its cwd to cwd-relative form and leaves outside paths absolute, so both forms are needed.
Add a read-only accessor on HiddenSet for its entries if none exists.
Hidden-path proof. Write an integration test that runs only when a real rg exists, and skips quietly otherwise. In a temp tree with a hidden folder and a hidden bare-name file, assert that no hidden content appears in the output. Cover plain use, a relative search, -g '*.txt', -u, -uuu, --no-ignore and --files. If you cannot make every one of those pass, STOP: leave rg unregistered, finish the other steps, and tell the user why.
zapcli core_commands/rg.rs and the router arm. Split the rest of the line with zap_wire::argv::split. Add a test that rg -n 'server_link|kick_send|identity|two_tab\b|start_server_link' /home/zakke/ProgStuff/x yields exactly three args, with the backslash-b intact. If the splitter mangles it, write a small POSIX-style splitter for rg. Reject args containing a newline. Run check and operand_refusal, then send rg <cwd>\n<arg per line>. Relative paths pass through; reads are exempt from the absolute-path rule. Add a test that first_word_is_chain_verb("rg foo\n") is false. Add rg to the help output.
zapgui terminal_router/rg.rs. Route rg requests to it from route. Re-run the same vetting, then find the binary with ZAP_RG, PATH, ~/.cargo/bin, ~/.local/bin, /usr/local/bin and /usr/bin, modelled on astgrep’s finder. Accept a binary only if --version starts with ripgrep. If none is found, reply with a one-line reason. Spawn with stdin null and the cwd the request carried. Kill after 10 s, with a “stopped after 10 s, narrow the search” note. Cap output at 100 KB with a “(output cut; narrow the search)” note. Exit code 1 means “no matches” (not an error); exit code 2 means errors, and stderr is included. The reply starts with === rg <args> ===.

Step 11. Failure hints suggest rg (small).
In play.rs, add a hint suffix inside the existing parenthetical. Failures are joined with “; “, so use a comma and never a semicolon or newline: ..., try: rg -nF '<pattern>' <folder>.

Replace, old text found 0 times. Add the suffix to the closest-line hint and to the “no line is close” hint, but not to the whitespace-match hint, where the fix is obvious. The pattern is the longest trimmed old-text line, cut to 60 characters. The folder is surface::project_root(path) if there is one, else the file’s parent.
Missing path with no did-you-mean. The hint becomes try: rg --files -g '*<file name>*' <nearest existing folder>. Skip it when that folder has two or fewer components, or the extension is a binary asset (png, jpg, jpeg, gif, webp, ico, pdf, zip, gz, tar, so, o, bin, exe, wasm, ttf, otf, woff, woff2, mp3, mp4).
Quoting. A small helper picks single quotes, falls back to double quotes if the text holds a single quote, and returns None (no suggestion) if it holds both kinds of quote or a backslash. The folder uses the same helper.
Unchanged. The huge-file case still gets no hint (its test expects ends_with("found 0")), and the did-you-mean string and the wrong-count hint are unchanged. Run the existing hint tests and fix only the ones that genuinely changed.

Step 12. Docs and todo.

CHAIN_CHEATSHEET.md: delete now takes a folder (no undo copy, set aside and purged after success, review shows counts); the rg verb with its limits; and a stronger Retry section saying that a failed chain applied nothing, that retry re-runs the whole chain, and that retry 3 plus one corrected block fixes step 3. The drift-guard test only checks verbs, so rg must appear there by hand.
README: a “Changes 2026-10-31” section.
RETRY.md: auto-apply for whitelisted retries.
CONTEXT_FOR_AI.md: dirwalk.rs, rg_gate.rs, terminal_router/rg.rs, core_commands/rg.rs.
NOTES_FOR_NEXT_AI.md: a “Batch 2026-10-31” section covering what was done, the decisions above and any skips.
todo.md: tick the items done and delete them, leaving the Zed item and the earlier open notes alone.

Things that must stay true: the shared parser stays the only parser; hidden always wins and is checked in every new path; always-reviewed verbs never auto-apply, including on retry; every chain write still needs an absolute path.

Finish with build_both.sh && zap. Your final note says what was done, what was skipped and why, and any design you adapted.
