# todo

## zapgui: confirm/deny on the diff viewer

- Add confirm and deny buttons (replacing the three do-nothing glass buttons).
- Confirm: write the new content to the path.
- Deny: copy something like `denied request to edit file <path>` to the clipboard.
- Writing is safe as-is: bytes go to disk, nothing interprets them. Content is visible in the diff before confirming, so no other guard needed on the body.

## zapgui: replace command

- Request shape:

      replace <path>
      <old text>
      %%%ZAP_SPLIT%%%
      <new text>

- Parse: first line is path, then old text up to the marker line, then new text to end of request.
- Match old text in the file in three layers, loosest last:
  1. exact match of the whole old text
  2. line-by-line with trailing whitespace trimmed on both sides
  3. line-by-line with leading and trailing whitespace trimmed on both sides (comparison only — real indentation in the file survives)
- Refuse on multiple matches. Refuse and report on zero matches.
- When a looser layer is used, say so in the reply so the AI knows its text wasn't exact.
- Feed the resulting new file content into the same diff pipeline overwrite uses, so confirm/deny works on it.

## zapcli: c commands

- Allow `c r`, `c c`, `c build` in zapcli.
- zapcli runs cargo itself; it does not forward to zapgui.
- On compiler error:
  - Fully automatic, no prompts.
  - Copy the compiler error text to the clipboard.
  - Also copy code chunks around each error location the compiler named.
  - User pastes once and the AI gets both the error and the relevant code.
- Needs new Outcome variants: something to carry the process result, and something that copies to clipboard on failure.

## zapcli: s and cdf

- Implement `s` (pick a project dir) and `cdf` (pick a folder in cwd).
- Both were fzf pickers in the real bashrc.
- fzf can't run inside zapcli — no real terminal for it to draw in.
- Replacement is a picker screen in zapcli's UI, not a reimplementation of fzf.

## zapcli: picker screen

- General mechanism: a mode field on the UI state. The draw function branches on it — Terminal or Picker.
- Terminal state and picker state both stay alive. The mode decides which gets drawn. Nothing is saved or restored, so nothing gets lost.
- Each picker owns its own state (list, filter text, highlighted entry, etc.).
- Picker needs three things from its caller: the list of options, a way to report the choice back, and a way to cancel.
- Filtering: substring to start with, add fuzzy scoring later only if it's annoying.
