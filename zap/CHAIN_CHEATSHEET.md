# zap cheat sheet

Paste this into any AI chat so it emits commands zapcli accepts.

## Commands zapcli understands

`ls`, `cd`, `cat` (also copies to clipboard), `s`/`settings`, `copy`,
`copy all`, `cls`/`clear`, `help`, `exit`/`quit`.

`head`, `tail`, `wc` — read part of a file. Output is violet and is not
included in `copy` (but is included in `copy all`).

`last [path]` — show the last change zap made to a file.

`whitelist` opens zapgui's menu of auto-apply folders.

`zap <text>` sends a line to zapgui.

`cargo <args>` runs cargo in the current folder and streams its output into
the bottom pane. Allowed subcommands: build, check, run, test, bench, clippy,
doc, clean, tree, version, the short forms b/c/d/r/t, and fmt only with
`--check`. Refused: install, publish, and the rest of cargo's write verbs;
`--config`, `--manifest-path`, `--target-dir`, `-Z...`, and any toolchain
other than stable/beta/nightly. An AI cannot start a run — only the human,
from zapcli's prompt, on a single line.

`create`, `replace`, `overwrite`, `rm`, `mv`, `mkdir` — see "Chains" below.

Paths: absolute, or `~/`, or relative to zapcli's current folder. No shell
expansion — `$VAR` is five literal characters.

## Chains

A chain is one paste into zapcli. One or more commands.

create, replace and overwrite have content between markers:

    create <path>
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

rm, mv and mkdir are one line, no body:

    rm <path> [<path>...]
    mv <from> <to>
    mkdir [-p] <path> [<path>...]

`replace` requires the old text to occur exactly once. `xN` means exactly N
times; all N are replaced. `create` fails if the file exists; `overwrite`
fails if it does not. `create` adds a trailing newline if the content does
not end with one. Missing parent folders are made when the chain is applied.

`rm` works on text files only, and only ones it can keep an undo copy of.
It refuses directories. `mv` never overwrites: to replace a file, `rm` it
first in the same chain. `mkdir` accepts `-p` and always creates parents.

Nothing in a chain is interpreted. Backticks, `$`, quotes, and heredoc
markers are all literal.

Markers are whole lines starting in column 0, trailing spaces ignored. The
line break right before a marker is not part of the text. If the content
itself contains a line like `@end`, use one extra `@` on every marker in
that command (`@@new`, `@@old`, `@@end`); the parser reads the `@` count
from the first marker.

Rules of thumb: keep the old text small and unique. One chain can touch many
files, and a step can target a file an earlier step made. All-or-nothing: if
any step cannot be done, nothing is written.

A chain with any `rm` or `mv` step always needs his review, even inside a
whitelisted folder. Whitelisted folders auto-apply only create, replace,
overwrite and mkdir.

## What you get back

Every decision — a step denied, Confirm, a failure, a whitelist auto-apply —
puts a plain sentence report on the clipboard and shows a short popup. A run
summary from `cargo` is appended under the last report if nothing else was
copied in between. One paste gives you the whole picture.