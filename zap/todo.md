# zap todo

- the assistant can't tell if the user already ran a command (touch, rm, etc.) or not, leading to unnecessary back-and-forth
- report cap: `report::build` currently concatenates every step's description with no length cap; long chains make a long report
- popup position and wording are first guesses; tune them after living with them a bit

## open — next round

- Report says more about which step failed. Today a chain failure names
  the path but not the step number, so a paste-back reads like "replace:
  expected 1 occurrence(s) of the old text in <path>, found 0" without
  saying it was step 3 of 7. Wrap the play-forward failure with the step
  index and verb so it reads like "step 3 of 7 (replace in <path>) failed:
  ...". Same for the failure sentence in `chain/report.rs`.

## open — chains with run steps

A chain should be able to include a cargo run (and a run of a plain file)
as a step, so "create a file, then build it, then replace based on the
build output" is one paste. Not started.

The easy part: the parser reads a header like `cargo test` or
`run x.sh` as a step, play-forward just records it, and the review window
shows it as a step with the command and folder. A chain with a run step
never takes the whitelist fast path, same rule as rm and mv: it always
goes to review, so an auto-applied edit is never followed by an automatic
build.

The real work is the connection. A chain today is request, reply, close,
then the user clicks confirm on a window whose socket is long gone. A
streaming step needs the socket to stay open through play-forward, review,
the writes, and the run, so the pane can receive frames the whole time.
That is the shape the plan doc calls a long-lived subscription, and it is
the reason this was deferred rather than done with the rest of item 1.

Questions to answer before starting:
- Does the run happen after the chain's files are written, or after every
  non-run step? (The plan says after the writes.)
- If a run fails, do the written files stay? (Probable answer: yes, they
  already passed review. The report should say so.)
- What goes on the clipboard: the chain report, the run summary, or both?
  (Probable answer: both, the run summary appended under the report, same
  as the standalone cargo verb.)

Wrap the failure report with the step index at the same time, since a
chain-run failure should read "step 4 of 7 (cargo test) failed: ...".

Estimated: an afternoon, most of it on the socket shape and the
review-then-stream sequencing.

- cat should support copying many files at once so I can get rid of the
  "for f" AI prompt and make it use zap instead.
- create an append verb, so adding a line to the bottom of a file does
  not need a full replace.
- rm can't touch a path with a space in it: the line is split on
  whitespace, so `rm a b` is two paths and `DELETE REQUEST,` becomes
  `DELETE` and `REQUEST,`. Need either one path per line, quoted paths,
  or only a single path per rm line.
- rename the rm verb to delete. `rm` typed in a terminal by accident is
  dangerous, and the guard against it only catches it because the command
  starts with `rm`.
- rework `s`. Today it's a shortcut for settings in zapcli; it should do
  what `s` does in my terminal instead. That's defined in my bashrc.
- add support for `cdf`, which is defined in my bashrc.
- word wrap on the input line is wrong. Typing a long path runs past the
  right edge instead of wrapping. Pasting does wrap, but the continuation
  lines don't start at column 0 — they start indented, leaving a gap on
  the left.
