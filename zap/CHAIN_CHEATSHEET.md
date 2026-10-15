# zap cheat sheet

Paste this into any AI chat so it emits commands zapcli accepts.

## Commands zapcli understands

`ls`, `cd`, `cat` (also copies to clipboard), `s`/`settings`, `copy`,
`copy all`, `cls`/`clear`, `help`, `exit`/`quit`.

`head`, `tail`, `wc` — read part of a file. Output is violet and is not
included in `copy` (but is included in `copy all`).

`last [path]` — show the last change zap made to a file.

`retry [<step>...]` — rebuild a chain from the last review's unapplied
steps and show it again. `retry 2` = step 2 of that chain, `retry 2 5`
= those two. After a Deny All the whole chain is retryable.

`whitelist` opens zapgui's menu of auto-apply folders.

`zap <text>` sends a line to zapgui.

`cargo <args>` runs cargo in the current folder and streams its output into
the bottom pane. Allowed subcommands: build, check, run, test, bench, clippy,
doc, clean, tree, version, the short forms b/c/d/r/t, and fmt only with
`--check`. Refused: install, publish, and the rest of cargo's write verbs;
`--config`, `--manifest-path`, `--target-dir`, `-Z...`, and any toolchain
other than stable/beta/nightly. The AI cannot start a run by itself. A run
can only be proposed as the last step of a chain (see below), and nothing
runs until the human confirms in zapgui.

`create`, `replace`, `overwrite`, `append`, `zaprm`, `zapmv`, `mkdir`,
`cd`/`cargo`/`run` — see "Chains" below.

## Paths

Chain writes require an absolute path or a path that starts with `~/`.
A bare relative path is rejected at parse time in zapcli, before anything
reaches zapgui, with a clear error. This is on purpose: the AI has no
reliable picture of zapcli's current folder, so a relative write is a
silent wrong-folder bug or a wasted round-trip. Reads (`cat`, `ls`,
`head`, `tail`, `wc`) stay relative-friendly.

No shell expansion — `$VAR` is five literal characters.

## Chains

A chain is one paste into zapcli. One or more commands.

create, replace, overwrite and append have content between markers:

    create <path> [--no-newline]
    @new
    <content>
    @end

    replace <path> [xN]
    @old
    <text to find>
    @new
    <text to put in its place>
    @end

    overwrite <path>
    @new
    <full new file content>
    @end

    append <path> [--no-newline]
    @new
    <text to append>
    @end

zaprm, zapmv, mkdir, rmdir are one line, no body:

    zaprm <path> [<path>...]
    zapmv <from> <to>
    mkdir [-p] <path> [<path>...]
    rmdir <path> [<path>...]

`replace` requires the old text to occur exactly once. `xN` means exactly N
times; all N are replaced. `create` fails if the file exists; `overwrite`
fails if it does not; `append` fails if the file does not exist. `create`
and `append` add a trailing newline if the content does not end with one,
and `append` adds a newline before the appended text if the file does not
already end with one. `--no-newline` turns both off. Missing parent folders
are made when the chain is applied.

`zaprm` works on text files only, and only ones it can keep an undo copy of.
It refuses directories. `zapmv` never overwrites: to replace a file, `zaprm` it
first in the same chain. `mkdir` accepts `-p` and always creates parents.
`rmdir` removes an empty folder; it fails if the folder has anything in it.

Nothing in a chain is interpreted. Backticks, `$`, quotes, and heredoc
markers are all literal.

## Markers and escapes

A marker is a whole line in column 0 that is exactly `@new`, `@old`, or
`@end`. Trailing spaces are ignored. The line break right before a marker
is not part of the text.

If the content itself contains a line that looks like a marker, escape it
by prefixing the `@` with one more `@`. So a content line `@end` is written
`@@end`, a content line `@new` is written `@@new`. Three `@` becomes
content `@@name`, and so on; the rule is "strip one leading `@`".

Only lines that actually collide with a marker need this. A patch hunk
header like `@@ -1,3 +1,4 @@`, an attribute like `@derive(Debug)`, or a
lone `@@` are content and pass through unchanged.

The older rule ("add an extra `@` to every marker in the command") no
longer works. Write markers as `@new`, `@old`, `@end`.

Rules of thumb: keep the old text small and unique. One chain can touch many
files, and a step can target a file an earlier step made. All-or-nothing: if
any step cannot be done, nothing is written.

A chain with any `zaprm`, `zapmv`, `cargo` or `run` step always needs review, even
inside a whitelisted folder. Whitelisted folders auto-apply only create,
replace, overwrite, append and mkdir.

## Running inside a chain

A chain may end with one run step, and it must be the last step. A run step
needs a `cd <folder>` line before it that sets the working folder:

    cd /home/me/proj
    cargo test --release

    cd /home/me/proj
    run /home/me/proj/scripts/build.sh arg1 arg2

The `cd` is a directive, not a step. It only applies to the run step after
it. `cargo` args pass the same allow-list as the standalone `cargo` verb.
`run` takes one file (absolute or `~/...`) plus optional args.

Nothing runs until the human confirms in zapgui. The review window shows
the run as one step with a RUN REQUEST banner for cargo, EXECUTE REQUEST
for run, and the usual trust checks still apply: if the target isn't
already trusted, a surface or execute review window appears first.

## cat

`cat a b c` copies every file to the clipboard in one blob, separated by
`=== /abs/path ===` headers. `cat a, b, c` also works — one trailing comma
per token is tolerated. Quotes cover spaces in paths: `cat "a b.txt" c.txt`.

## What you get back

Every decision — a step denied, Confirm, a failure, a whitelist auto-apply —
puts a plain sentence report on the clipboard and shows a short popup. A run
summary from `cargo` is appended under the last report if nothing else was
copied in between. One paste gives you the whole picture.

A failure report names the step index and verb, so a paste-back locates the
problem without a follow-up: e.g.
`zap: nothing was written. step 3 of 7 (replace /abs/x.rs) failed: expected 1 occurrence(s) of the old text in /abs/x.rs, found 0.`