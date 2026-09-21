# zutil — context doc

Workspace: ~/ProgStuff/egui/zutil
Crates: app, core, db, design, error_stuff, json_parsing, logging, ui_math
Deps: db_wrapper + protocol + db from ~/ProgStuff/z_db via absolute paths

Reusable components live in a **separate workspace**:
~/ProgStuff/egui/reusable_components
Crates there: popup, fading_popup, textbox, pulse, bounce_text, close_button, debug_log, scrollable_text_input

Run from zutil root: `c run` (alias for ./local_cargo.sh, which sets RUSTFLAGS to avoid cranelift/egui SIMD issues).
Tests: `./run_all_tests.sh`.
Build a .deb and install it: `./build_and_install.sh`.

---

## What zutil is

A personal knowledge tool. You keep texts (notes, snippets, prompts, terminal commands) and templates (fill-in-the-blank text with %%markers%%). You browse them through a circle menu, copy them to the clipboard, or fill them in and copy the result.

The window is a borderless always-on-top panel. You bring it forward with a global shortcut.

---

## Shape of the app

The root screen is a circle. Each item on the circle is a "nav". Nav items are built from one of:

- **Templates** — categories with templates → templates → fill overlay
- **AI Prompts** — every text with `type_of_text = "prompt"`, flat list, click copies
- **Text** — categories with non-prompt texts → texts → view overlay
- **Terminal Commands** — same as AI Prompts but filters `type_of_text = "terminal command"`
- **Projects** — projects, click opens a terminal at its path

Clicking a box that has children pushes a new circle onto the stack. Right-click on the box opens an editor. Right-click on empty circle space pops one level.

A top menu bar sits at the top with View all / Projects / Json / New + / Half ctrs. Those open full-screen pages inside a frame.

The window has a close X in the top right that minimizes it (does not exit).

Every box on a circle carries:
- a counter box on the right showing a running click count
- a shortcut box on the left, empty until you right-click it and set one

Shortcuts fire while the item's circle is on screen. No global shortcuts.

---

## Schema

Current version: 7. The enum lives in `db/src/migration/schema_versions.rs`. Each version is a file in `db/src/migration/schemas/versionN.rs` that returns the complete table set for that version, and knows how to migrate from version N-1.

`entire_table()` from the newest version defines what a fresh DB looks like. `update_from_old_version` on each intermediate version does the incremental upgrade.

### categories
- `id` — primary key
- `name` — the display name
- `type_of_category` — `"normal"` or `"meta"`. Normal categories show in the normal picker; meta only in the meta picker. Same name can exist once per type.

### texts
- `id` — primary key
- `title` — display name, shown on the circle box
- `body` — the content. This is what gets copied on a prompt/terminal click
- `category_id` — nullable FK to categories. The specific group the text belongs to
- `meta_category_id` — nullable FK to categories. A broader group
- `type_of_text` — nullable free string. Examples: `"prompt"`, `"terminal command"`, `"notes"`, `"shortcuts"`, `"url"`, `"text"`. Nothing in the app treats these as fixed values except the AI Prompts and Terminal Commands navs, which filter on `"prompt"` and `"terminal command"` respectively
- `copy_instead_of_view` — 0/1. If 1, clicking this text in a circle nav copies
  the body to the clipboard instead of opening the view overlay. Defaults to 0 (view).
- `popularity_ctr` — incremented every time this text is clicked

### templates
- `id` — primary key
- `title` — display name
- `content` — the template body. Contains `%%marker%%` placeholders
- `instructions` — explains what each marker should contain. Optional
- `example` — what a filled-in version looks like. Optional
- `category_id` — nullable FK to categories
- `meta_category_id` — nullable FK to categories
- `popularity_ctr` — same as texts

### template_fills
- `id` — primary key
- `template_id` — FK to templates
- `label` — optional short label for this saved fill
- `values_json` — the filled-in values as a JSON blob
No UI uses this yet.

### keyword_lookup
- `id` — primary key
- `text_id` — FK to texts
- `keyword` — a keyword associated with a text
Used only for the FTS5 index. No UI reads it.

### logs
- `id` — primary key
- `timestamp` — millis since epoch
- `level` — free string, e.g. `"info"`, `"error"`
- `category` — free string, e.g. `"ui"`
- `source` — free string, e.g. `"json_import"`
- `session_id` — in-memory id, unique per app launch
- `message` — the log line
- `details` — optional blob
- `details_type` — optional type tag for `details`
Written on JSON import. Nothing else logs.

### key_value_storage
- `id` — primary key
- `key` — unique
- `value` — the stored string
Not used by any UI yet. Reserved for settings.

### projects
- `id` — primary key
- `title` — display name
- `path` — folder path, expanded `~` is stored
- `launch_zed` — 0/1, whether a "zed" button shows
- `launch_adstud` — 0/1, same for "adstud"
- `popularity_ctr` — same as texts

### main_nav_clicks
- `id` — primary key
- `name` — matches a main-nav label exactly ("Templates", "AI Prompts", "Text", "Terminal Commands", "Projects")
- `popularity_ctr` — click count for that nav item

**IMPORTANT**: if you add or rename a main-nav item, update `MAIN_NAV_NAMES` in `db/src/helpers/popularity.rs`. `init_db` creates a row for each name at startup.

### shortcuts
- `id` — primary key
- `owner_kind` — what the shortcut belongs to. One of: `"main_nav"`, `"category"`, `"text"`, `"template"`, `"project"`
- `owner_id` — the identifier for that owner. For `main_nav` it's the label; for the others, the numeric id as a string
- `combo` — the shortcut string, e.g. `"ctrl+shift+t"` or `"z"`. **Not unique** — different circles can reuse the same combo, since only one circle is on screen at a time
Same name can exist once per type.

### error_log
- `id` — primary key
- `timestamp` — millis since epoch
- `screen` — the `unwrap_or_bail!` screen name
- `location` — the `unwrap_or_bail!` location string
- `detail` — the error text
Written when the disabled screen shows an error. One row per distinct error per run.

### schema_version
- `id` — primary key
- `version` — the current schema version as an integer

---

## JSON import / export

Three block types. The `parsing_instruct` field picks which:

- `"new_texts"` — items are `{ title, body, category?, meta_category?, type_of_text? }`
- `"new_templates"` — items are `{ title, content, instructions?, example?, category?, meta_category? }`
- `"new_categories"` — items are `{ name, type_of_category? }` (`"normal"` or `"meta"`, defaults to `"normal"`)

Categories referenced by name get created automatically. Import runs in a transaction. If any title collides with an existing row, the popup offers Replace / Keep both / Cancel.

Export builds the same shape from the current DB. Validate parses without importing.

The prompt text the app hands out lives in `json_parsing/src/prompt.rs`.

---

## Where things live

`app/src/app.rs` — the `App` struct, all state, the main `ui()` loop.

`app/src/globals.rs` — menu stack, last click position, modal-open flag, right-click disabled flag. Shared across the app without threading through every call.

`app/src/components/` — components that are specific to zutil (circle_menu, top_menu, page_frame, back_button, category_picker_popup, text_input, shortcut_picker, background, close_button, bounce_text, pulse). Some are copies of things from `reusable_components`.

`app/src/ui_screens/` — full-screen pages and overlays. `final_nav_view_or_edit_modals/` holds the ones the circle menu opens.

`db/src/helpers/` — every DB call. No UI types here. Categories, texts, templates, projects, popularity, shortcuts, error_log, import, export.

`design/src/colors.rs` — all colors used anywhere in the UI.
`design/src/numbers.rs` — all padding, sizes, durations.

---

## Architecture rules the user cares about

- **Errors**: every fallible call uses `unwrap_or_bail!(result, "screen", "location")` from `error_stuff`. On failure it flips a global `APP_DISABLED` atomic, stores an `AppError { detail, screen, location }`, and `App::ui` renders `disabled::ui::disabled_ui` instead of the page. The disabled page shows the error, recent logs for the current session, and a copy button.
- **No code in mod.rs** — only `pub mod` declarations. Hard rule.
- **Test style** is documented in `~/ProgStuff/z_db/db_wrapper/tests/how_test_should_be_written.rs` (has an "AI NOTES" section at the bottom). Read it before writing tests.
- **`db` crate is pure data** — no UI types, no json_parsing types except where explicitly crossed (json import helpers take `NewText` / `NewTemplate`, done knowingly).
- **No `.unwrap()` outside tests** except in `init.rs` (startup) and `error_stuff` itself.
- **Multi-column edits go in a transaction.** Use `begin_all_or_nothing` / `everything_went_perfectly` / `regret_everything`, matching `import_payloads`.
- **Colors and numbers go in the `design` crate.** Don't hardcode them in ui code.
- Workspace lints set `warnings = "allow"`.

---

## Working style (from the user's own doc, "WORKFLOW FOR WORKING WITH ME")

- Talk phase → plan phase → "go" → code. No code before go.
- When they shift back to asking questions, STOP giving code. Even mid-edit.
- Short answers. No walls of text.
- Don't offer options when one is obviously right.
- Don't preemptively solve the next thing.
- If unsure, say so and ask. Don't guess.

---

## What's unresolved / half-done

- The `design` crate exists but `app.rs` and most `ui_screens` still have hardcoded colors and sizes inline.
- `template_fills` table exists but no UI uses it.
- `key_value_storage` table exists but no UI uses it.
- `search.rs` helpers are dead code — the View all page filters client-side.
- No filter UI for `type_of_text` (other than the AI Prompts and Terminal Commands navs).
- The disabled screen has no re-enable / retry button.
- `logs` is only written on JSON import. Nothing else logs.
- Projects shows in two places (circle nav and top menu), each with its own code path.
