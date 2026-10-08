# zutil

A personal knowledge tool. Texts (notes, prompts, terminal commands) and
templates (fill-in-the-blank text with %%markers%%), browsed through a circle
menu. Clicking copies to the clipboard or opens a view/edit overlay.

Window is borderless, always-on-top, starts minimized in release builds.
Brought forward with a global shortcut.

## Crates

Workspace members: `app`, `db`, `json_parsing`, `logging`, `design`,
`error_stuff`, `ui_math`, `core`.

External deps via absolute paths:
- `protocol`, `db_wrapper`, `db` from `~/ProgStuff/z_db`
- `popup`, `textbox`, `pulse`, `bounce_text`, `close_button` from
  `~/ProgStuff/egui/reusable_components`

## Build and install

./build_and_install.sh

Builds a .deb (app + zutil_popup) and installs it via dpkg. Needs sudo.

## Run

Installed binary: `~/.local/bin/zutil`.

## Test

./run_all_tests.sh

## new.sh

Makes a copy of this project next to it with a name you type. Skips target,
.git, Cargo.lock.

## local_cargo.sh

I have nightly with cranelift and other stuff that egui doesn't work with.
I don't want to change my global settings, so I run this instead for building.

If I'm in the project root, my `c` alias checks for local_cargo.sh in the
current folder and uses it if it's there. Otherwise it falls back to plain
`cargo`. So `c run`, `c build`, etc just work.

If not in the root, use the file directly:

./local_cargo.sh                # run
./local_cargo.sh build
./local_cargo.sh build -p app
./local_cargo.sh add serde
./local_cargo.sh clean

## Docs

- `temp_doc.md` — context doc for the codebase
- `zutiltodo.md` — pending work

## Keybind management

Right-click a box's shortcut slot to open the picker. Press a key combo to
set it, Enter to save, Escape or right-click to cancel. Backspace or Delete
clears the combo. Enter with an empty combo on an item that already had one
clears the keybind.

## Categories are case-insensitive

Creating or referencing a category whose name differs only by case from an
existing one fails with a clear error. Renaming has the same check. Any
pre-existing collisions are reported at startup in a popup and logged.

## Export

The JSON screen's Export button now produces a SQL script (INSERT statements
for categories, texts, templates, keyword_lookup, template_fills, and
main_nav_clicks). The text editor's "Copy everything" button produces the
same shape, scoped to one text.
