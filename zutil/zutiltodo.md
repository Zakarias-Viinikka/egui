# zutil todo

## done recently

- z_db build fixes (`Col::Null`, `ForeignKeyDef` renames).
- Schema moved to the per-table style (`db/src/tables/tbl_*.rs`, `SchemaColumn`
  constants). Migration chain preserved; tests pass.
- Minimize on launch (release builds only).
- Categories are case-insensitive on create / reference / rename.
- Collision report at startup for pre-existing case-duplicates.
- Text editor: Delete (with confirm), Copy everything (SQL scoped to the text).
- JSON screen Export now produces SQL, not JSON.
- Shortcut picker: Backspace/Delete clears the combo; Enter with empty on an
  item that had one clears the keybind.
- Prompt handed out by the JSON screen is now dynamic and lists the DB's
  actual categories.

## what the user wants changed

- **Copy categories button.** A button next to copy prompt that copies the db's categories plus a few examples, so the AI gets a picture of what categories and tables exist.
- **Category edit is broken.** Investigate and fix.
- **Routing example.** The user made a routing example and isn't sure if it's finished. zutil's routing needs to change to follow it. Ask for the file.
- **Move components.** `app/src/components/` should live in an `every_page_ever` crate inside the zutil workspace, same shape zap has.
- **Main menu routing.** Buttons should take you to tables. Remove the hardcoded thing that takes you to some specific weirdo place.
- **Minimize on launch.** App should minimize on load without the egui freeze. Doesn't need to happen instantly, just eventually on load.
- **Popularity ordering.** Most-used buttons should sort a certain way instead of by creation order. (Ask user for details if unclear.)
- **New table: tmp notes.** A place to jot notes down. Page has a plus button in the middle. Only asks for title and content, nothing else.
- **Modal restyle.** The user doesn't like the current modal design. Prefers something closer to how the "edit keybind to button" editor feels — like it's more part of the background.
- **Bigger circle, top menu to the side.** The circle menu should be bigger; top menu moves to the side to make room.
- **Update the json thing.** Details TBD — ask user.
- **Creating a new thing as a human.** Should feel bigger and look nicer, and selecting a category should be easier.
- **Categories get descriptions.** So future-me can see what a category is for, and so the AI gets context.
- **Combine copy prompt and copy db.** Currently two buttons; should be one.
- **Left click + ctrl hide.** If you go back enough times, the app should hide.
- **Auto nav back to main menu.** Should return to the main menu automatically on certain events. User needs to figure out which events.

## open questions

- Which file is the routing example?
- Which events trigger the auto-return-to-main-menu?
- What does "most used buttons order" mean exactly — descending count, frequency-windowed, something else?
