# fading_popup

A small rounded box with text in it that fades out and exits. It appears
top-center, sits for 800ms, fades over 500ms, and exits. Click it and it
exits immediately.

It's a **binary**, not a library. You run it as a program.

    fading_popup "copied" <x> <y>

x and y are the top-left corner of the popup in screen pixels.

## How to build it

Always build with local_cargo.sh from the workspace root. Never use bare
cargo.

    cd ~/ProgStuff/egui/reusable_components
    ./local_cargo.sh build --release -p fading_popup

### Why

This user's global cargo config (~/.cargo/config.toml) forces the cranelift
codegen backend:

    [target.x86_64-unknown-linux-gnu]
    rustflags = ["-Zcodegen-backend=cranelift", "-C", "link-arg=-fuse-ld=mold"]

cranelift cannot compile llvm.x86.sse.cvtss2si, which is the float-to-int
instruction that fontdue uses when rasterizing glyphs. If you build
fading_popup with bare cargo, the binary compiles fine but panics the first
time it tries to render text:

    llvm.x86.sse.cvtss2si is not yet supported.

local_cargo.sh sets RUSTFLAGS in the environment, which overrides the
config's rustflags list for that invocation, so cranelift isn't passed to
rustc. The popup then renders.

If you see the panic at runtime, you built with the wrong backend. Rebuild
with local_cargo.sh.

## Why this isn't an egui component

Two X11 problems make an in-process popup unusable on XFCE.

Focus. eframe uses GDK on Linux, which advertises WM_TAKE_FOCUS on every
window it creates. xfwm4 reads that and gives the new window keyboard focus.
Anything you were typing into stops receiving keys. winit has this open
since 2019 (winit issues #1160, #4479) and confirms there's no option to map
a window without focusing it. eframe's with_active(false) is documented as
Unsupported on X11.

Races. Even if you try to reclaim focus with an external command, you're
racing xfwm4. Sometimes it wins. That's why earlier attempts flickered.

The fix, which xfce4-notifyd uses for its notification popups, is to make
the window override-redirect. An override-redirect window is one the window
manager doesn't manage at all: no focus, no taskbar, no stacking rules, no
restacking. That removes the race instead of trying to win it.

override_redirect must be set at window creation time, and eframe/winit
doesn't expose it. So this popup is written directly against X11 with the
x11rb crate.

## What it does

- override_redirect — the WM never sees it
- 32-bit ARGB visual — per-pixel alpha for smooth rounded corners
- _NET_WM_STATE_ABOVE — stays on top of other windows
- _NET_WM_STATE_SKIP_TASKBAR and _SKIP_PAGER — never in taskbars
- _NET_WM_WINDOW_OPACITY — fade-out at the end
- fontdue — anti-aliased text at 26px

## What it isn't

No decorations, no close button, no resize handle, no input other than
click-to-dismiss. It's a notification, not a dialog.

## If you want to change how it looks

Colors, size, corner radius, linger time, and font size are constants at the
top of src/main.rs. Edit and rebuild. No config file.

## If you're calling it from another project

The other project needs to know where the fading_popup binary is. Common
approaches:

- Install it to a known location (e.g. /usr/bin/zutil_popup) and spawn that
  path.
- Put it next to the calling binary and spawn it relative to the current
  executable.
- Copy the crate into the calling project's workspace and build it there.

Whatever the approach, the calling project also needs to build fading_popup
with local_cargo.sh or with RUSTFLAGS set, or the popup will panic on text
render. See "How to build it" above.
