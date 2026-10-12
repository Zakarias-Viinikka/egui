# todo — immediate

Everything from the previous immediate list is done except the items
below. Details are Claude's call.

## viewer follow-ups

- Word wrap arrow color is bright orange and stands out. If it turns out
  too loud in practice, tone it down. Tune after living with it.

## chain internals

- The move-detection thresholds (DICE_FLOOR, ALIGN_FLOOR, LONG_RUN) are
  first guesses. Watch for false positives or missed moves and tune the
  named constants in text_diff/moves.rs.

## zapcli

- Chain writes still require absolute paths and this stays. If reads
  ever cause a wrong-folder surprise, revisit.
- [ ] Give the right-side glass buttons distinct colors based on the type of action or request they represent, so different operations are visually distinguishable.
- [ ] Add a dedicated "Allow Special Request" confirmation button for execution requests such as `run` and `cargo`. These requests must not be approved by the ordinary green confirmation button. Give the associated glass panel a distinct colored border or accent to make special requests visually obvious.
- [ ] Add a verification button for ordinary file reviews, allowing the user to explicitly mark a file as verified and move on without requiring special-request approval.
- [ ] Fix copying errors to the clipboard, which is currently broken.
