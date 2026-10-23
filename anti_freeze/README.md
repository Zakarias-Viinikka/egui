# anti_freeze

A hidden egui app that watches RAM and caps the top memory hogs when the
system gets close to swapping. Starts invisible. No taskbar entry.

## What it does

A background thread samples `MemAvailable` from `/proc/meminfo`. When used
memory crosses 90%, it trips.

On trip: shows a popup saying "RAM cap engaged" and applies a cgroup memory
limit (via the sudo helper at `/usr/local/bin/anti_freeze_cap`) to the top
five memory-consuming programs. The trip does not raise the window.

A watcher thread then wakes every 60 seconds, checks whether usage has
dropped below 75%, and lifts the cap when it has. If it hasn't, sleeps
another 60 and checks again. No fixed duration, no ceiling.

The cap is the default anti-freeze behaviour. It runs from startup
regardless of whether the window has ever been shown.

## The GUI

Super+Page_Up toggles the window. First press brings it up fullscreen with
focus, second press hides it. On launch, the window does not show at all —
the first frame sends `Fullscreen(false)` and `Visible(false)` before
drawing anything else.

The list of top memory processes is refreshed while the window is visible:
on show (focus change) and every 5 seconds after. Nothing is refreshed
while the window is hidden.

Freeze is currently disabled. The `Freeze`/`Unfreeze` button, its click
handler, and the row-click confirmation are commented out in
`freeze_button.rs`, `modal.rs`, and `ram_list.rs`. The backing code
(`killing.rs`, `watchdog.rs`, the `frozen_pids` field) is still there so it
can be turned back on later, but nothing in the running GUI calls it.

## The watchdog

`main` spawns a separate `watchdog` binary that outlives us. If anti_freeze
crashes while processes are frozen, the watchdog resumes them. The list of
frozen pids is in `/tmp/anti_freeze_frozen_<our_pid>`.

Since freeze is disabled, the watchdog currently has nothing to do. The
binary must be installed at `~/.local/bin/watchdog` or `main` prints a
warning on every launch. That warning is expected when `watchdog` isn't
installed.

## Renaming the binary

The crate is still named `app`, but `app/Cargo.toml` declares
`[[bin]] name = "anti_freeze"`, so the built binary is
`target/release/anti_freeze`. `build_and_install.sh` installs it as
`~/.local/bin/anti_freeze` and kills any running copy first.

## What it does not do

It does not track whether the capped programs are behaving. "Safe to lift"
only means memory usage dropped. Nothing here can see whether a program is
going to spike again.

---

# egui_template

## new.sh

makes a copy of this template next to it with a name you type.
skips target, .git, Cargo.lock.

./new.sh

## local_cargo.sh

I have nightly with cranelift and other stuff that egui doesn't work with.
I don't want to change my global settings, so I run this instead for building.

If I'm in the project root, I don't need to do anything different — my `c`
alias checks for local_cargo.sh in the current folder and uses it if it's
there. Otherwise it falls back to plain `cargo`. So `c run`, `c build`, etc
just work.

If I'm not in the root, use the file directly:

./local_cargo.sh                # run
./local_cargo.sh build
./local_cargo.sh build -p core
./local_cargo.sh add serde
./local_cargo.sh clean
