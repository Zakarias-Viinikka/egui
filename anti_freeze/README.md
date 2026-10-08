# anti_freeze

A hidden egui app that watches RAM and caps the top memory hogs when the
system gets close to swapping. Starts invisible. No taskbar entry.

## What it does

A background thread samples `MemAvailable` from `/proc/meminfo`. When used
memory crosses 90%, it trips.

On trip: shows a popup saying "RAM cap engaged" and applies a cgroup memory
limit (via the sudo helper at `/usr/local/bin/anti_freeze_cap`) to the top
five memory-consuming programs. No freeze, no window raise — the trip is
silent apart from the popup.

A watcher thread then wakes every 60 seconds, checks whether usage has
dropped below 75%, and lifts the cap when it has. If it hasn't, sleeps
another 60 and checks again. No fixed duration, no ceiling.

## The GUI

Open it with Super+Page_Up. What it's for:

- See the top memory processes
- Right-click a row to end it (with a confirm)
- Freeze/Unfreeze button — manual. Freezing SIGSTOPs every other process
  you own, except the ones in `killing::NEVER_STOP` (the WM, session, and
  the watchdog). Unfreezing resumes them.

The trip does not use the freeze button. Freeze is only ever a manual
action.

Closing the GUI (Super+Page_Up again) automatically unfreezes anything
frozen, so you can't leave the system stuck.

## The watchdog

`main` spawns a separate `watchdog` binary that outlives us. If anti_freeze
crashes while processes are frozen, the watchdog resumes them. The list of
frozen pids is in `/tmp/anti_freeze_frozen_<our_pid>`.

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
