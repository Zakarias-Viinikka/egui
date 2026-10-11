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