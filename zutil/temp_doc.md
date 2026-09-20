# zutil — context doc

Workspace: ~/ProgStuff/egui/zutil
Crates: app, core, db, error_stuff, json_parsing, logging, ui_math
Deps: db_wrapper + protocol + db from ~/ProgStuff/z_db via absolute paths

Reusable components live in a **separate workspace**:
~/ProgStuff/egui/reusable_components
Crates there: popup, fading_popup

Run from zutil root: `c run` (alias for ./local_cargo.sh, which sets RUSTFLAGS to avoid cranelift/egui SIMD issues).
Tests: `./run_all_tests.sh`.

---

## Answers to your five questions

**1. Main menu items.**
The version I have in memory had: "View all", "Projects", "Json". If it now says "Templates, AI Prompts, Text", that's a newer change I don't have context on — ask the user, not me.

**2. `type_of_text`.**
Free string, nullable. Not an enum. Set manually on create/edit, or via JSON import. Example values used so far: `terminal command`, `notes`, `prompt`, `shortcuts`, `url`, `text`. Purpose is filtering/grouping — nothing in the app switches behaviour on it.

**3. `category_id` vs `meta_category_id`.**
Both nullable FKs to `categories.id`.
- `category_id` = the specific grouping ("adb commands", "git", "android").
- `meta_category_id` = a broader grouping ("ai", "docs").
The `categories` table has a `type_of_category` column ("normal" or "meta"). Normal categories only appear in the normal picker, meta only in the meta picker. Same name can exist once per type (name is not unique on its own — but historically there was a UNIQUE constraint on name that was manually dropped via SQL; current schema has no UNIQUE).
There is no rule forcing `meta_category_id` to point at a meta-type category — the UI just restricts the picker.

**4. Should "Text" show every row in texts?**
The version I have showed everything (texts + templates) on one "View all" page, with a search box that filters client-side across title, category, meta_category, and type_of_text. If it's been split into "Text" vs other items, that's newer — ask.

**5. Uncategorized bucket.**
Never decided. Nothing in the code hides uncategorized texts. They currently appear in the flat list.

---

## Schema (version 0 is the only real version — everything was folded in, only a couple of migrations exist for dev convenience)

Tables:
- **categories**: id, name, type_of_category ("normal" | "meta")
- **texts**: id, title, body, category_id (FK), meta_category_id (FK), type_of_text
- **templates**: id, title, content, instructions, example, category_id (FK), meta_category_id (FK)
- **template_fills**: id, template_id (FK), label, values_json
- **keyword_lookup**: id, text_id (FK), keyword
- **logs**: id, timestamp, level, category, source, session_id, message, details, details_type
- **key_value_storage**: id, key (unique), value
- **projects**: id, title, path, launch_zed, launch_adstud
- **schema_version**: id, version

Templates use `%%name%%` markers inside `content`. `example` is what a filled-in one looks like. `instructions` says what to fill markers with.

---

## Architecture rules the user cares about

- **Errors**: every fallible call uses `unwrap_or_bail!(result, "screen", "location")` from `error_stuff`. On failure it flips a global `APP_DISABLED` atomic, stores an `AppError { detail, screen, location }`, and `App::ui` renders `disabled::ui::disabled_ui` instead of the page. The disabled page shows the error, recent logs for the current session, and a copy button.
- **No code in mod.rs** — only `pub mod` declarations. Hard rule.
- **Test style** is documented in `~/ProgStuff/z_db/db_wrapper/tests/how_test_should_be_written.rs` (has an "AI NOTES" section at the bottom). Read it before writing tests.
- **`db` crate is pure data** — no UI types, no json_parsing types except where explicitly crossed (json import helpers take `NewText` / `NewTemplate`, done knowingly).
- **No `.unwrap()` outside tests** except in `init.rs` (startup) and `error_stuff` itself.
- Workspace lints set `warnings = "allow"`.

---

## Working style (from the user's own doc, "WORKFLOW FOR WORKING WITH ME")

- Talk phase → plan phase → "go" → code. No code before go.
- When they shift back to asking questions, STOP giving code. Even mid-edit.
- Short answers. No walls of text. No "let me break this down."
- Don't offer options when one is obviously right.
- Don't preemptively solve the next thing.
- If unsure, say so and ask. Don't guess.

---

## What exists in the app right now (as of my last state)

Pages:
- Main menu
- View all (list of texts + templates, search, edit/delete/fill)
- Create text / edit single text
- Create template / edit single template / fill template
- Json (paste JSON, pick Import/Export/Validate, copy prompt, example page)
- Projects (title opens a terminal at the path, optional "zed"/"adstud" buttons, edit/delete)
- Disabled (error screen)

JSON import supports three instructs:
- `new_texts`
- `new_templates`
- `new_categories`

Import runs inside a transaction (`db::transactions`). On success a fading popup shows counts. Export builds the same JSON shape from current data.

Logging crate writes to `logs` on every JSON import. Nothing else logs yet. Session id is in-memory (`OnceLock`), so it resets each app launch — intentional, so logs are grouped by run.

---

## Things I know are unresolved / half-done

- No re-enable / retry button on the disabled page.
- `template_fills` table exists but no UI uses it yet.
- `search.rs` helpers (search_texts_by_title, search_templates_by_title) are dead code — the view-all page loads everything and filters client-side.
- No filter UI for `type_of_text`.
- `seed_db` / test coverage for categories / projects / templates beyond schema shape is minimal.
