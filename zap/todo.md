# todo - immediate

Each item has a one-line done-when.

## zapcli

- `s` and `cdf`: both blocked until the user pastes the two bashrc
  definitions. Done when each does what it does in the bashrc.
- `<verb> --help` copies a short description to the clipboard, so the
  AI can tell the user "run `<verb> --help`". Done when `--help` after
  every chain verb puts a paragraph on the clipboard.
- Built-ins `cat`, `head`, `tail`, `wc`, `ls` do not accept quoted
  paths with spaces. Done when a quoted path with a space works for
  each of them.
- Chain `cd` and the create / replace / overwrite / append path lines
  take no quotes. Done when a quoted path with a space is accepted
  there too, or the cheat sheet says why it can't be.
- `edit` command: fzf-pick a file, then open it in Mousepad. Done when
  `edit` opens the picked file.

## viewer

- Wrap-arrow colour, to tune after living with it. Done when the arrow
  reads as a marker, not a distraction, in real diffs.
- The preview shown for a step still includes the edit of an earlier
  marked step on the same file. The WRITE is correct now (see
  writer::apply_detailed); the preview is cumulative. Done when the
  preview for a step reflects only the wanted steps.
- Move-detection thresholds (DICE_FLOOR, ALIGN_FLOOR, LONG_RUN) are
  first guesses. Done when they are tuned from actual false-positive
  and missed-move cases.
- Copy selection from the right-click menu: the 14a spike found no
  public accessor on egui's label selection state, so a selection
  cannot be read from the menu after the click. Done when either a way
  to read the selection is found and an entry is added, or the item is
  deleted as impossible.