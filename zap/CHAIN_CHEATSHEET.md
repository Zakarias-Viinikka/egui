# zap cheat sheet

Last updated: 2026-10-25

Paste into any AI chat so it emits commands zapcli accepts.

## Commands

ls, cd, cat (also copies to clipboard), s / settings, copy, copy all,
cls / clear, help, exit / quit.

head, tail, wc - read part of a file. Output is violet, skipped by copy.

last [path] - show the last change zap made to a file.

retry [<n>...] - rebuild a chain from the last review's unapplied steps.

whitelist - zapgui's menu of auto-apply folders.

zap <text> - send a line to zapgui.

cargo <args> - runs cargo in the current folder, streams output into the
bottom pane. Allowed: build, check, run, test, bench, clippy, doc, clean,
tree, version, short forms b / c / d / r / t, fmt only with --check.
Refused: install, publish, other write verbs; --config, --manifest-path,
--target-dir, -Z..., toolchains other than stable / beta / nightly. A
run can only be proposed as the last step of a chain. Nothing runs until
the human confirms in zapgui.

## Paths

Chain writes need an absolute path or one starting ~/. A bare relative
path is rejected in zapcli before anything reaches zapgui - the AI has
no reliable picture of zapcli's current folder. Reads stay
relative-friendly.

No shell expansion - $VAR is five literal characters.

## Chains

A chain is one paste. One or more commands.

create, replace, overwrite, append carry content between markers:

    create <path> [--no-newline]
    @new
    <content>
    @end

    replace <path> [xN]
    @old
    <find>
    @new
    <replace-with>
    @end

    overwrite <path>
    @new
    <full file>
    @end

    append <path> [--no-newline]
    @new
    <text>
    @end

zaprm, zapmv, mkdir, rmdir are one line:

    zaprm <path> [<path>...]
    zapmv <from> <to>
    mkdir [-p] <path> [<path>...]
    rmdir <path> [<path>...]

replace needs the old text to occur exactly once. xN means exactly N
times; all N are replaced. create fails if the file exists; overwrite
fails if it doesn't; append fails if it doesn't. create and append add a
trailing newline if missing; append also adds one before the appended
text if the file doesn't end with one. --no-newline turns both off.
Missing parent folders are made.

zaprm works on text files only (undo copy kept); refuses directories.
zapmv never overwrites. mkdir accepts -p and always creates parents.
rmdir removes an empty folder only.

Nothing in a chain is interpreted. Backticks, $, quotes, heredoc markers
are literal.

## Markers and escapes

A marker is a whole line in column 0 that is exactly @new, @old, or
@end. Trailing spaces ignored. The line break before a marker is not
part of the text.

A content line that looks like a marker gets one extra @: @end becomes
@@end, @new becomes @@new, three @ is content @@name, etc. Rule: strip
one leading @. Only lines that collide with a marker word need this.
@@ -1,3 +1,4 @@ and @derive(Debug) pass through.

## Review

A chain with any zaprm, zapmv, cargo or run step always needs review,
even inside a whitelisted folder. Whitelisted folders auto-apply only
create, replace, overwrite, append and mkdir. Unapproved .sh / .toml
files also go to review - see FILE_TRUST.md.

## Running inside a chain

A chain may end with one run step, and it must be last. It needs a
cd <folder> line before it:

    cd /home/me/proj
    cargo test --release

    cd /home/me/proj
    run /home/me/proj/scripts/build.sh arg1 arg2

cd is a directive, not a step. cargo args pass the same allow-list as
the standalone cargo verb. run takes one file (absolute or ~/...) plus
optional args. Nothing runs until confirmed.

## cat

cat a b c copies every file to the clipboard in one blob with
=== /abs/path === headers. cat a, b, c also works (trailing comma
tolerated). Quotes cover spaces.

## What you get back

Every decision (deny, Confirm, failure, whitelist auto-apply) puts a
plain sentence report on the clipboard and shows a popup. A cargo run
summary appends under the last report.

A failure report names the step index and verb:
zap: nothing was written. step 3 of 7 (replace /abs/x.rs) failed: ...