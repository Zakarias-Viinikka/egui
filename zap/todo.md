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
when cat fails to find a file, it also shows in the cli terminal

- [ ] `edit` command: fzf-pick a file like the normal terminal `run`/`cdf` picker, then open the selected file in Mousepad.

##zapgui permission bug
basically when there is a request appended at the end. i have to confirm the request then when i click confirm, it does a new popup for just the request that i have to confirm again. id prefer if it just made me do it once, and the glass panels on the right were color coded so i could easily see when i need to accept them.

also i feel like a system where isntead of just having to pick apply or deny, i could have a third option where i click "pause" where whatever i selected as "deny" is in mode "this is what the ai will edit" it's complicated to explain but the idea is basically. i somehow express "everythign was fine excepts the files i denied" something gets copied to my clipboard, then the ai im working with can express "ok that gives me the info i need to know which ones i need to change" and it gives me a command like "zap try again becauser user denied but wanted to redo not deny all" and it only needs to change the things i denied, and it knows what i denied and what i want it to do because it's explained in what gets automatically put in clipboard
