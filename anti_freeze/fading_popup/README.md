# fading_popup

Copied from `~/ProgStuff/egui/reusable_components/fading_popup`.

A tiny standalone X11 program that draws a rounded box with text,
lingers 800ms, fades out, and exits. No egui, no eframe. Draws
straight to X11 with x11rb, rasterizes text with fontdue.

**It's a binary, not a library.** It's a separate process on purpose —
if the app that spawned it crashes, the popup stays alive and can
still show info.

Usage:

    fading_popup "some text" <x> <y>

x and y are the top-left corner of the popup in screen pixels.

## Calling it from anti_freeze

`core::popup::show(msg, x, y)` spawns the binary as a subprocess. The
function looks for the binary next to the current executable
(`target/debug/fading_popup`), so it just works when you run
`c run` from the workspace root.

## Don't build this with bare cargo

Your global cargo config forces the cranelift backend, which can't
compile the float-to-int instruction fontdue uses. The binary compiles
fine but panics the first time it renders text:

    llvm.x86.sse.cvtss2si is not yet supported.

Use `./local_cargo.sh` from the workspace root. It sets RUSTFLAGS in
the environment, which overrides the global config for that run.
