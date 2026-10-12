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

- show an indicator of the max edit boundary — something like "edits are only happening in /home/zakke/ProgStuff/suggestion_for_leptos_book/" so it's visible at a glance that nothing above that path is being touched

- rename the rm and mv verbs to zaprm and zapmv so that pasting a zap command into a real terminal by mistake doesn't run a real rm or mv

- the smart diff logic is broken. an edit shows up as a delete plus an insert instead of an edit. the program should be able to tell the difference. it also shouldn't label as "delete and edit" something that just moved down because code got inserted or removed above it — that's a natural shift, not a delete.

- overwriting a file with content identical to what's already there shouldn't fail the chain. that step should just be skipped and the rest of the chain should continue.
if a command failed because one thing wasn't right. there should be some kind of system so the ai doesn't have to rewrite the entier thing. instad it can do like "retry" command or soemthing and it tries to rerun the previous thing and it only has to rewrite the ones that failed
