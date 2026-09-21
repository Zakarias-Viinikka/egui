# Potential performance issues

Not urgent. The app runs fine. But if it ever feels sluggish, these are the
first places to look.

## Menu items get rebuilt and cloned a lot

The menu stack lives in globals.rs, behind a mutex, as a Vec<Vec<MenuItem>>.
Every call to current_items() locks the mutex and clones the entire
outer Vec, including every MenuItem inside.

Every frame of circle_menu_ui calls current_items() at least once to get
the items it's about to draw. Every call site that reads shortcuts, or
dispatching clicks, or firing shortcuts, calls it again. During a single
frame that's several full clones of a data structure that can hold tens of
items, each with strings and options inside.

What to do if it matters:

- Cache the current items in App as a field, and only refresh it when the
  menu stack actually changes (push, pop, rebuild).
- Or change current_items() to return a reference, using a RwLock instead
  of Mutex so multiple readers don't fight.
- Or split the global into a small handle with cheap accessors, avoiding
  clones entirely.

The cost today is measured in microseconds per frame. It only starts to
matter if menus get very large.

## Menu is rebuilt on every counter bump

bump_counter_for() used to call rebuild_menu_stack() after every click.
That rebuilt every level of the menu from the DB, which meant a full read
of texts, templates, categories, projects, plus a shortcut lookup per item.

This was removed because the visible effect was a full redraw that made
the circle flicker on every click. If you ever need counters to update
live while you're looking at them, don't put the rebuild back inline.
Instead, mark the menu as stale and rebuild at most once per N frames,
or update only the counter numbers in place.

## egui repaints continuously when animations are alive

The circle menu requests a repaint every frame while a pulse is running or
a fade-in is in progress. That's correct — without it, egui would stop
drawing and animations would freeze. But it means a continuous 60Hz redraw
of the whole window for the duration.

Not a problem at current scale. If it becomes one, the fix is to shorten
the pulse and fade durations, or to stop requesting repaints when the
window isn't focused.

## DB reads are synchronous

Every read (read_all_texts, read_all_templates, etc.) is a blocking call
on the UI thread. On a local sqlite file that's fast, but each menu build
does several of them. If menus ever start to feel slow on a large DB, the
reads are what's growing, not the UI.

To diagnose: add timing around load_rows / push_* methods and log the
milliseconds. If DB reads dominate, cache them per-menu-build instead of
per-item.

## Not an issue yet, don't fix

- Fading popups spawn a separate process each time. Process spawn is a few
  milliseconds. You'd need many popups per second for this to matter.
- The shortcut picker and its descendant chain read the current items
  repeatedly. Same as above.
- egui widgets do their own layout every frame. That's how egui works.
  Nothing to do about it here.


---

# Performance logging (proposed, not built)

If menus ever get slow, the fastest way to find out why is to log how
long each piece takes and query the log. Here's the design so a future
AI can build it in one pass.

## Proposed table

New schema version, adds performance_log:

| column | type | meaning |
|---|---|---|
| id | INTEGER | primary key |
| timestamp | INTEGER | millis since unix epoch |
| area | TEXT | coarse bucket, see below |
| label | TEXT | specific operation name |
| duration_us | INTEGER | how long it took, in microseconds |
| context | TEXT | free-form, e.g. n_items=42 or table=texts |

area is the broad category, label is the specific call. Areas to use:

| area | what it covers |
|---|---|
| db_read | any read from sqlite (read_all_texts, read_category_name_by_id, ...) |
| db_write | any insert or edit |
| menu_build | push_* methods that build a menu level |
| menu_globals | current_items(), pop_menu(), rebuild_menu_stack() |
| render | per-frame UI code if you ever want to time it |
| shortcut | firing a shortcut, including descend chains |
| popup | spawning and rendering a popup |

## Proposed helper

A function like this, so timing is one call:

    pub fn time_it<T>(
        db: &LiveForever,
        area: &str,
        label: &str,
        context: &str,
        f: impl FnOnce() -> T,
    ) -> T

It runs f, measures wall time, and writes a performance_log row with the
elapsed microseconds. On by default in debug builds, off in release unless
an env var like ZUTIL_PERF=1 is set -- otherwise the log grows fast and
the writes themselves become the cost.

## How to use it

Wrap the thing you suspect and run the app for a few minutes:

    let rows = time_it(db, "db_read", "read_all_texts", "", || read_all_texts(db));

Then query the log for the slowest areas:

    SELECT area, label, COUNT(*) AS n, AVG(duration_us) AS avg_us,
           MAX(duration_us) AS max_us
    FROM performance_log
    WHERE timestamp > (strftime('%s','now')*1000) - 600000
    GROUP BY area, label
    ORDER BY avg_us DESC
    LIMIT 20;

The area with the biggest avg_us and the biggest n is where the time is
going. n matters as much as avg_us -- a slow call that runs once per
second matters less than a fast call that runs 200 times per frame.

## What to do with the answer

- If a db_read dominates and runs per item, cache it.
- If menu_globals dominates and the main cost is cloning, cache the item
  list in App and stop calling current_items().
- If menu_build dominates on every click, batch or lazy-build.
- If nothing shows up but the UI still feels slow, it's egui repainting.
  Check how often request_repaint is called and whether anything keeps
  it spinning.

## Caveats

- Microsecond resolution is fine for this. Don't over-engineer.
- Writes are themselves DB operations. Log every Nth call if you're
  worried about the logger becoming the bottleneck.
- The DB file is at ~/.local/share/zutil/zutil.sqlite. Table is small
  enough to leave untruncated for a long time.
