# CLAUDE_PLAN.md

The brief handed to the implementing AI. Written down so it survives
the session.

---

You're working in the zap workspace at ~/ProgStuff/egui/zap/. Read
README.md, CHAIN_CHEATSHEET.md, CONTEXT_FOR_AI.md, NOTES_FOR_NEXT_AI.md
and todo.md first. A stronger AI has already made every design decision
for this pass, including the ones the earlier brief left as "your call".
Implement them as written, don't reopen them, and work through the whole
pass without pausing between items. Where this brief disagrees with
todo.md or an earlier brief, this one wins. If something fails
repeatedly, stop and ask the user. Talk to the user in short plain
prose: no bullets, no metaphors, no jargon.

GROUND RULES. egui-src/ is the vendored egui 0.36.2: don't edit it,
never cargo update, and look up exact egui API names there instead of
guessing from memory. Don't change DiffRow, RowKind or Side. zapgui,
zapcli and zap_wire are separate workspaces, and any change in zap_wire
means both binaries must be rebuilt (build_both.sh). If you can run
cargo, export RUSTFLAGS="-C link-arg=-fuse-ld=mold" first, as
build_both.sh does, and run cargo check or cargo test in each workspace
after each item; if you can't, write carefully, because the user builds.
After you change Step or Verb, the compiler's errors are your list of
places to update. Add unit tests wherever a pure function changes
(parser, apply_step, pairing, moves, frames).

ORDER. Shared parser, popups, cat, routing, failure report, deny-red;
then the viewer as one design; then edit-vs-replace and move detection;
run steps last. The viewer items (word wrap, per-side bars, selection,
move strips, full-file toggle) all change what a row is, so build the
row drawing once, as described below, even if you land it in several
edits.

SHARED PARSER. Move parse(), ParseError, Step and Verb out of
zapgui/app_core/src/chain into a new zap_wire::chain module, tests
included, and keep zapgui's chain/parser.rs and chain/step.rs as
one-line re-exports so existing imports still compile. zapcli will call
the same parse() before it sends anything. That is how the absolute-path
rule and the cargo allow-list get enforced on both sides without a
second parser. Step gains args (Vec<String>), cwd (Option<String>) and
no_newline (bool). Verb gains Append, Cargo and Run. Parse errors name
the command they belong to (its header text and line number).

MARKER ESCAPE (changed from todo.md). A marker is a whole line in
column 0 that is exactly @new, @old or @end (trailing spaces ignored).
In a body, a line made of two or more @ followed by exactly new, old or
end is an escape: strip one @ and keep it as content, so @@end becomes
@end and @@@end becomes @@end. Every other line is content, untouched,
including @derive(Debug), @@ -1,3 +1,4 @@ and a lone @@. This is
deliberately narrower than "strip one @ from every @@ line": patch hunk
headers and decorators must never be altered silently, and only the
three names that collide with a marker need escaping. The old "extra @
on every marker" form now fails loudly with "expected @new". Replace
the old escape test with one test per case above.

APPEND AND --NO-NEWLINE. append <path> [--no-newline] with an
@new/@end body, shaped like create. The file must exist. Default: if the
file is non-empty and has no final newline, add one first, then the
body, then a trailing newline if the body lacks one. --no-newline is
accepted on create and append only, as a trailing token on the header
line (same trick as replace's xN), and turns all of that off: create
writes the body exactly (empty body gives an empty file), append just
concatenates. Append's newline logic lives in play.rs apply_step because
it depends on the file's current content. Create's trailing-newline rule
stays in the parser and is skipped when no_newline is set. Appending
nothing is the usual "would not change" error. Wire append through
writer.rs like create/replace/overwrite, report.rs, last_change
(Edited), the UI's verb names, zapcli's verb list and help, and the
cheatsheet.

ABSOLUTE PATHS. In the shared parser, every path of create, replace,
overwrite, append, rm, mv (both), mkdir, plus run's file and cd's
folder, must start with / or be ~ or ~/... . Anything else is a
ParseError such as "create: path must be absolute or start with ~/
(got src/a.rs)". zapcli parses first and prints a red "zap: nothing was
sent. <error>" without touching the socket. zapgui re-parses and
enforces the same rule. Reads (cat, ls, head, tail, wc) stay
relative-friendly.

ROUTING AND THE OVERWRITE BUG. Likely cause:
first_word_is_chain_verb requires the raw input to start with the verb,
so a paste with a leading blank line or space returns false. create and
replace then fail as unknown commands, but overwrite falls into route()'s
"overwrite " branch, the bare handler, which succeeds silently and
writes the markers into the file. Don't patch the heuristic; remove the
second path. Delete zapcli's core_commands/overwrite.rs and the
"overwrite " branch in route(). Send any input whose first non-blank
line starts with append, create, replace, overwrite, rm, mv or mkdir
through chain.rs, judged on the input with leading whitespace trimmed. A
marker-less overwrite then fails in the parser with "expected a marker
line", which is loud instead of corrupting a file. Add a router test
that "overwrite /home/x/a.rs\n@new\nhi\n@end\n" reaches chain.rs. Also
test it with a leading newline. zapgui's bare handle_overwrite stays.
Delete BUG_OVERWRITE_ABS_PATH.md afterwards. Also in route(): a block of
two or more non-blank lines whose first word is cd, cargo or run is a
chain (see RUN STEPS). Single-line cd, cargo and run stay the human
verbs.

CAT. cat.rs already handles several files, but only comma-separated.
"cat a b c", which is what an AI writes, becomes one path with spaces
and fails. Give cat the raw text after the word cat (route() currently
passes whitespace-split args), split it with zap_wire::argv::split
(quotes cover spaces in paths), strip one trailing comma from a token,
and keep the rest as is: one clipboard blob, === /abs/path === headers.

POPUPS. In popup.rs look for fading_popup (not popup_demo_app) in
~/.local/bin, then PATH. Call it as fading_popup "text" x y with no host
argument, and set the size constants to the real 260x60 so the centering
maths is right. In build_both.sh, before the pkill/restart, build
fading_popup from ~/ProgStuff/egui/reusable_components (cargo build
--release -p fading_popup) and install it to
~/.local/bin/fading_popup. The exported RUSTFLAGS is enough, no
local_cargo.sh needed. Cargo uses the RUSTFLAGS variable instead of the
rustflags in ~/.cargo/config.toml, so the cranelift flag never reaches
rustc, which is why zapgui already builds. Verify once by running the
installed binary and seeing text render. Only if it panics with
llvm.x86.sse.cvtss2si, build that one crate through local_cargo.sh
instead.

FAILURE REPORT. play_forward and writer::apply must say which step
failed. In play_forward, move the per-step body into a helper returning
Result<AppliedStep, String> and wrap its errors in the loop as "step 3
of 7 (replace /abs/x.rs) failed: <reason>". In writer::apply wrap
execute_step the same way, and for the snapshot-changed check name the
first step that touches that file. report::build keeps its sentence
shape ("zap: nothing was written. step 3 of 7 (...) failed: ..."). Drop
leading "replace:"-style prefixes from the reasons where they now read
twice.

DENY RED. In glass_button/ui.rs change FILL_DENIED and BORDER_DENIED to
translucent red derived from RED (210,80,80). They are premultiplied, so
rgb must not exceed alpha: (91,35,35,110) for the fill and (181,69,69,220)
for the border work. The UI currently picks Denied over Selected, so the
shown step would lose its highlight. Add a DeniedSelected BlockState
(same red, brighter border) used when the shown step is denied.

VIEWER (word wrap, selection, move strips). Draw rows in three passes.
Pass 0, before the scroll area: lay out each side's text as a galley
from a LayoutJob in monospace. Wrap width is that side's inner width
minus gutter, bar, padding and about 14px for the scroll bar. Turn
break_anywhere on so long tokens wrap instead of overflowing. Row height
is max(ROW_H, the taller galley plus padding), plus a TAG_H strip (about
16) on top when a move span starts on that row.
Pass 1, inside the ScrollArea: allocate each row at its computed height
(the hr above Change blocks stays) and paint with the painter: bars
spanning the row's full height below any strip, line numbers anchored to
the first wrapped line, and the strip's label.
Pass 2: draw the left-column text of every row, then the right-column
text of every row. Each is an egui Label built from its pre-laid galley
with selectable(true), placed in a child Ui with a top-left layout. Do
not use ui.put or add_sized, both centre their widget. Finishing the
whole left column before any right-column label matters: egui's label
selection runs across labels in the order they were added (check
text_selection in egui-src), so this keeps a drag inside one column
instead of interleaving both. Copy then works through egui's own
selection and Ctrl+C. Confirm it reaches the clipboard; if it doesn't,
catch Event::Copy and send the selected text through
app_core::clipboard. If a view has more than about 1500 rows, only
create labels for rows inside ui.clip_rect(); selection then covers
rendered rows only, which is acceptable.
Give the ScrollArea an id built from the step index and the full-file
mode so scroll position isn't shared between steps. Bundle the new
inputs (a MoveSpan slice, the scroll id, the one-shot scroll flag) in a
small params struct instead of growing the argument list. Reviews
without moves pass an empty slice. Create the MoveSpan type now (see
MOVE DETECTION), even though detection comes later.

BARS. Choose bar colour per side. Delete: left red. Insert: right green.
Change: left red and right green (drop the blue). Equal: none. A row
inside a move span: violet on the side the span lives on, instead of red
or green.

CAPTION AND GEOMETRY. First pull the duplicated geometry maths of the
chain page and draw_surface_review into one helper, because caption
height, the full-file pill and the run banners all change it. Then lay
the caption out wrapped, its galley height replacing the constant
caption_h in both places, rendered as a selectable label so a path can
be copied.

FULL-FILE VIEW. A button, not a click on the diff (a click would fight
text selection and the mis-click guard). It is a small glass-style pill
labelled "full file" / "changes only" at the top right of the viewer
area, outside the scrolling content; add a function for it in
glass_button/ui.rs using the existing fill and border constants. State
is one bool in the zapgui UI struct for the whole review, reset when a
new chain arrives. No new data is needed. AppliedStep already holds the
complete before and after text of each step. For surface reviews use the
SurfaceDiff old and new text; check surface_review.rs and keep the texts
on the review's file entries if it drops them. Add pair_lines_full, the
same pairing with the context limit removed (iterate diff.ops() instead
of grouped_ops(3), sharing the loop body). Run its output through
process() lazily the first time a step is shown in full mode, cache it
per step in the UI struct, and clear the cache when the chain changes.
When switching to full mode, scroll once so the first change sits a few
rows below the top (one-shot flag into the viewer, offset from the
pass-0 row heights).

EDIT VS REPLACE (pair.rs). Same function signature, same types, Equal
ops untouched. For every other op (deleted plus inserted lines), align
old and new lines by similarity instead of by index.
Similarity of two lines: take runs of letters, digits and underscore as
tokens and ignore everything else (whitespace and punctuation match
between unrelated lines and inflate scores). Weight each token by its
length, find the weighted longest common subsequence of the two token
lists, and score 2 * matched weight / (total weight of both lines). Two
lines with no tokens at all are compared the same way on their
non-whitespace characters. A pair counts as similar at or above
SIMILARITY_THRESHOLD = 0.5 (named constant).
Alignment: a dynamic program over the k old and j new lines that
maximises total similarity, where a pair may only be matched if it is
similar; then backtrack. Skip pairs that cannot reach the threshold from
token weights alone (score <= 2*min/(sum)). When k*j exceeds 100_000,
skip the program and pair by index with the same threshold test.
Output: a matched pair is one row (Equal if same_text, else Change). Old
lines left unmatched between two matches become Delete rows, then the
unmatched new lines become Insert rows: all deletes of a gap before all
inserts, never interleaved.
Tests: comma added gives Change; a line swapped for an unrelated one
gives Delete then Insert; renamed variable gives Change; re-indented
line gives Change; three old lines against four new with one inserted in
the middle stay aligned.

MOVE DETECTION (new text_diff/moves.rs, row types untouched). Spans are
in file line numbers, not row positions, so they work in the snippet
view and the full-file view alike. MoveSpan: role (From = old-file lines
on the Delete side, To = new-file lines on the Insert side), first_line,
last_line, label (String), edited (bool). Detection runs over every step
of the chain at once, so code moved between files (rm plus create, or
extracted into a new module) is found too.
Input: the flat pair_lines rows of each step. play_forward keeps them
while building steps, runs detection once after all steps are played,
stores each step's Vec<MoveSpan> in a new AppliedStep field, and only
then turns the flat rows into UI blocks with process() as today. Run and
mkdir steps contribute no rows.
Candidates: maximal runs of consecutive Delete-only rows and of
Insert-only rows. Ignore runs with fewer than 3 non-blank lines or fewer
than about 60 non-whitespace characters. Never pair a Delete run with an
Insert run that touches it directly; that is a replacement.
Matching: first a cheap score (Dice over the multisets of
letter/digit/underscore tokens, at least 0.6), then confirm with the
alignment function from edit-vs-replace (matched weight over the longer
run's weight, at least 0.7). If either run exceeds about 300 lines, skip
the alignment and require Dice 0.8. Take pairs greedily by descending
score, each run used once. edited means the matched lines aren't all
identical after trimming.
Labels are built here, where paths are known. On the From span: "moved
to line 120" (same step) or "moved to src/foo.rs:120" (other step). On
the To span: "moved from ...". Add " (edited)" when edited.
Visual: violet bars on both ends, plus the thin strip above the first
row of the span showing the label in violet, on the side the span lives
on. The viewer decides membership from line numbers only: a Delete row
whose left line number is inside a From span, an Insert row whose right
line number is inside a To span. Never colour anything by inspecting row
text.

RUN STEPS IN CHAINS.
Grammar. Two new single-line chain steps, "cargo <args>" and
"run <file> [args]", plus a "cd <folder>" line that is not a step: it
sets the working folder for the run step after it. The parser enforces:
- a run step needs a cd before it (the AI can't know zapcli's folder,
  same reason as absolute paths);
- at most one run step per chain, and it is the last step (one
  open/done sequence per connection is all the pane handles);
- the cd folder and the run file are absolute or ~/...;
- cargo args pass zap_wire::cargo_gate::check.
The docs sentence "An AI cannot start a run" is no longer true. Replace
it with: an AI can propose one run as the last step of a chain, nothing
runs until the human confirms in zapgui, and the cargo allow-list and
surface/run trust checks still apply. Also add cargo_gate::check at the
top of cargo_run::run: its doc says zapgui's check is the one that
counts, but nothing calls it there.

Review. Add Chain::needs_review() (any Remove, Move, Cargo or Run step)
and use it in handle_chain instead of has_remove_or_move, so a run chain
never takes the whitelist path. In play_forward a run step becomes an
AppliedStep whose path is the resolved folder and whose after is a
two-line command text ("cargo test --release" / "in /abs/folder"),
pushed through pair_lines and process the way mkdir's pseudo text is.
writer::execute_step does nothing for it. The UI shows the RUN REQUEST
banner for cargo and EXECUTE REQUEST for run. The never-looked-at guard
now covers run steps too (update its message). report.rs words them
"will run ..." and "would have run ...".

Wire. New FrameKind::ChainRun = 0x05 (zapcli to zapgui, text
"<cwd>\n<block>" like the chain request), with Frame::chain_run and
from_byte updated and the round-trip test extended. In zapcli,
core_commands/chain.rs parses first. If the chain has a run step it
returns Outcome::Streaming from a new socket::open_chain_run, otherwise
the old send(). Fold the three near-identical open_cargo, open_run and
open_chain_run bodies into one helper that takes the frame. Check the
stream-drain code in zapcli's drawing_requirements.rs. A session that
ends without ever opening the pane (denied, parse error, withdrawn) must
print its Reply line as a normal zap line and unlock the input when the
connection closes. Add that if it is missing. A plain chain request that
contains a run step gets a one-line error reply from zapgui.

zapgui handler (route_requests::handle_chain_run, dispatched from the
listener on FrameKind::ChainRun). It runs on the connection's own thread
and blocks for the life of the chain.
1. Start one watcher thread that reads frames from a clone of the stream
and sets an Arc<AtomicBool> cancel flag on Cancel, EOF or any error. Add
an optional cancel flag to stream_run::StreamSpec, and as a parameter of
cargo_run::run and run_file::run (None for the standalone verbs), so the
run uses that flag instead of spawning a second reader. Two readers on
one socket steal each other's frames.
2. Parse and play_forward as handle_chain does (errors: clipboard,
popup, and a Reply frame with the sentence).
3. Create an mpsc channel and put its Sender on the Chain (new field
notify: Option<Sender<ChainOutcome>>). ChainOutcome is Applied with the
wanted flags, AllDenied, or Replaced. Wrap the chain in an Arc and stash
it through a new stash_pending_arc that returns the displaced Arc, so
the handler keeps its own Arc for withdrawing. A displaced chain's
notify gets Replaced (send after releasing the slot lock).
4. Wait in a loop of recv_timeout(100ms). If the cancel flag is set or
600 seconds pass, call clear_pending(&mine) and send a one-line Reply.
5. On Applied: if the run step is wanted, call cargo_run::run or
run_file::run with the step's folder and the cancel flag. They already
do trust and surface review, stream the output and append the summary.
If the run step was denied, reply "applied n step(s), run denied". On
AllDenied or Replaced, reply a one-line sentence.
Send nothing on the stream before the run starts, so the run's own
StreamOpen is the only one.

UI side. When the pending chain disappears without the UI having
cleared it (withdrawn), hide the window. On a successful confirm and on
all-denied, after the existing report, popup and clear_pending, call a
small finish method on the chain that sends the outcome. A failed write
is not terminal: the window stays as it does today and nothing is sent.

Trust and report. A run target the chain itself wrote (a create, replace,
overwrite or append step on the same path) was already reviewed as a
diff. After a successful apply and before sending Applied, save a
run_trust entry (canonical path, args, sha, text) for it, the same idea
as refresh_trust_after_chain_confirm does for cargo surface files.
Anything else goes through the normal trust check when the run starts
and may open its own review window; that is fine. The chain's report
must reach the clipboard as RecordKind::Report (check copy_to_clipboard
does that) so stream_run's summary appends under it. That is why the
order is writes, run, then summary, and one paste gives the whole
picture.

DOCS AND REPORT. Update README.md (wire protocol including ChainRun and
the blocking handler, chain grammar and escape rule, viewer, where files
go), CHAIN_CHEATSHEET.md (escape rule, append, --no-newline, absolute
paths, cd/cargo/run steps, multi-file cat), NOTES_FOR_NEXT_AI.md (done
list) and zapcli's help text. Remove finished items from todo.md and
leave nothing stale. Finish with a short report to the user: what you
did, what you left and why, and anywhere you deviated from this brief.
