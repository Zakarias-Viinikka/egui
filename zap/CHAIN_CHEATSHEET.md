# zap chains: cheat sheet

Paste this into any AI chat so it emits chains that zapcli accepts.

## What a chain is

A chain is one paste into zapcli. It contains one or more commands, each
introduced by a header line, with the file content between marker lines.
zapcli forwards the whole block to zapgui, which either applies it (if all
files are whitelisted) or shows a diff to review.

Nothing in a chain is interpreted by a shell. Backticks, `$`, quotes and
heredoc markers are all literal.

## The three verbs

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

`replace` requires the old text to occur exactly once. `xN` means exactly N
times; all N are replaced. `create` fails if the file already exists;
`overwrite` fails if it does not. `create` adds a trailing newline if the
content does not end with one. Missing parent folders are made when the
chain is applied.

## Paths

Absolute, or starting with `~/`, or relative to zapcli's current directory.
Everything after the verb up to end-of-line is the path, so spaces are fine.
No `$VAR` — zapcli does not expand shell variables.

## Markers

Markers are whole lines starting in column 0. Trailing spaces on the marker
line are ignored. The line break right before a marker is not part of the
text. If your content itself contains a line like `@end`, use one extra `@`
on every marker in that command (`@@new`, `@@old`, `@@end`); the parser reads
the `@` count from the first marker and only lines with exactly that count
count as markers.

## Rules of thumb

- Keep the old text small and unique. `fn old() {}` is safer than a whole
  function body if the function name is distinctive.
- One chain can touch many files, and a replace can target a file an earlier
  create made. Steps see each other's results.
- All-or-nothing: if any step cannot be done, nothing is written.
- Chains are applied as a whole, so put dependent steps in order.

## Reading the pasted-back report

After Confirm, Deny, a failure, or a whitelist auto-apply, the clipboard
holds a plain sentence report. It names each file and what happened. A
failure says `nothing was written` and gives the reason; anything after that
is context for what the chain would have done.

Popups carry short text only, so they are not the report.
