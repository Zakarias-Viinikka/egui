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

Hidden paths in ~/.config/zap/hidden.txt are refused everywhere: any
chain step that touches one, cd into a hidden folder, run on a hidden
file, and every read built-in (cat, head, tail, wc, ls, cd, last). The
refusal line is "deliberately hidden by the user: <path>".

## Allowed extensions

~/.config/zap/extensions.txt (honours XDG_CONFIG_HOME) is an opt-in
allowlist. A missing or empty file allows every extension. When the
list is set, create, replace, overwrite and append only write a path
whose final component has one of the listed extensions. Extensionless
paths (Makefile, .gitignore, foo.) are refused. zaprm, zapmv, mkdir,
rmdir and chmod are not checked, so a rename can change the extension.
run and cargo bypass it. Add or remove entries from the whitelist menu
(`whitelist` in zapcli), under Allowed extensions.

A token is 1 to 16 letters, digits, `_`, `-` or `+`, one leading dot
ignored, lowercase. If the file has tokens but none is valid, every
checked verb is refused with a message telling the user to fix or
delete the file. The error names the offending extension and lists
up to 12 allowed ones.

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

zaprm, zapmv, mkdir, rmdir are one line; chmod +x <path>... makes one
or more files executable (only +x, one step per path):

    zaprm <path> [<path>...]
    zapmv <from> <to>
    mkdir [-p] <path> [<path>...]
    rmdir <path> [<path>...]
    chmod +x <path> [<path>...]

rename moves a file inside its folder. The new name is a bare file
name, no slash, not . or .., and must differ from the old one. zapmv is
still the verb for a cross-folder move.

    rename <path> <new-name>

replace needs the old text to occur exactly once. xN means exactly N
times; all N are replaced. create fails if the file exists; overwrite
fails if it doesn't; append fails if it doesn't. create and append add a
trailing newline if missing; append also adds one before the appended
text if the file doesn't end with one. --no-newline turns both off.
Missing parent folders are made.

zaprm works on text files only (undo copy kept); refuses directories.
zapmv never overwrites. mkdir accepts -p and always creates parents.
rmdir removes an empty folder only.

Quoted paths with spaces work for zaprm, zapmv, mkdir, rmdir, chmod and
rename: write the whole path in quotes. They do not work for create,
replace, overwrite or append, which take everything after the verb as
the path; those handle a space in an unquoted path but not the quotes
themselves, and the failure shows the quote in the path. A quoted path
with spaces in a quoted verb like zaprm that is not quoted splits into
two tokens, and the second is relative, so the parse fails loudly
instead of touching the wrong file.

Shell operators (&&, |, ;) are not supported. cd takes one folder and
refuses anything with those in it. A cd inside a chain that contains
them is rejected with "cd: &&, ; and | are not supported; put cd on its
own line and the cargo or run line under it".

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

A chain with any zaprm, zapmv, chmod, cargo or run step always needs
review, even inside a whitelisted folder. Whitelisted folders auto-apply
only create, replace, overwrite, append and mkdir. Unapproved .sh /
.toml files also go to review - see FILE_TRUST.md.

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
summary appends under the last report. Popup text ends with ` · copied`
or ` · copy failed`.

The report is capped at 8000 bytes. The header (verdict plus failure
reason) is always written in full. Denied and Marked steps are listed
first; Applied steps fill what's left. Anything cut is counted at the
end as `(+ N denied/marked step(s) not listed)` or `(+ N applied
step(s) not listed)`.

A failure report names the step index and verb, and the reason may end
with one parenthesised hint taken from the text the step actually saw:
zap: nothing was written. step 3 of 7 (replace /abs/x.rs) failed: expected 1 occurrence(s) of the old text in /abs/x.rs, found 0 (line 41 looks closest: `let total = 1;`).

Hints you can get: the old text matches at line N when indentation, tabs
or line endings are ignored; the closest line (or, for several old
lines, the first old-text line that differs); the line numbers of up to
five matches when the count is wrong; "use overwrite" when create hits an
existing file; a similar file name, or the nearest existing folder, when
a path does not exist. Files over 20,000 lines get no hint.

A chain that does not parse is rejected as a whole, and every problem
found is listed in one sentence, in line order, each clause ending with
"at line N" (at most 20, then "(+ N more)"):
zap: nothing was sent. use zaprm at line 2; create: path must be absolute or start with ~/ (got src/a.rs) at line 5; cd: folder must be absolute or start with ~/ (got tmp) at line 9.

When the reviewer used the edit-title pen (create, mkdir, zapmv,
rename steps only), the report also carries one clause per edited
target: the old path, the new one, and how many later steps were
updated to match.

## astgrep rename

astgrep rename <folder> <old> <new>

Renames the Rust identifier <old> to <new> in every .rs file under <folder>, using the installed ast-grep binary. One line, four tokens. <folder> is absolute or starts with ~/. <old> and <new> are plain ASCII identifiers (letter or underscore first, then letters, digits, underscore), not `_` alone, not a Rust keyword, and they must differ. Lifetimes and loop labels are never renamed. Files under target, .git and dot-folders are skipped.

Always goes to review. At most one astgrep step per chain. A trailing cd + cargo is fine. Every other step in the chain must stay off the .rs files under the folder (before astgrep) and off the files astgrep changed (after it), or the chain fails with "put it in a separate paste". The extension allowlist does not apply. If ast-grep is missing or too old, the step fails with a one-line reason. ast-grep is found through ZAP_AST_GREP, PATH, ~/.cargo/bin, ~/.local/bin, /usr/local/bin, /usr/bin.
