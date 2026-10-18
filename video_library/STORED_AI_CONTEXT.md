# video_library — AI stored context

Written by AI for AI. Read this first. If code and this file disagree,
the code wins and the file should be corrected. Update a section when
its content changes and bump that section's date.

## What this is — 2026-10-18

A Rust + egui desktop app for video editing assistance. Pull a
selection from Kdenlive, work on it, push it back. Play video, scrub a
timeline, apply effects, track objects.

Two workspaces:

- `/home/zakke/ProgStuff/egui/video_library/` — the app.
- `/home/zakke/ProgStuff/egui/video_library_lab/` — a lab crate
  `draw_lab` used to prototype drawing, guessing, and tracking before
  it moves into the main app. It depends on `app_core` from the main
  workspace by path, so shared code (decoder, proxy, error) is one
  copy.

The tracker is Cutie (Python, PyTorch), driven from Rust as a
subprocess. Kdenlive integration is D-Bus against a patched fork.
Neither is optional — both are how the tool actually works.

## Crate map — 2026-10-18

Main workspace:

- `app` — binary, wires eframe.
- `every_page_ever` — all UI code.
- `app_core` — all logic, no egui. Includes `video/decoder`,
  `video/proxy`, `video/decode_worker`, `state/*`,
  `kdenlive/{pull,push,talker}`, `thumbnails/*`, `popup/*`.
- `video_library_core_core` — video effects. Currently just
  `black_and_white/core.rs`.

Lab workspace:

- `draw_lab` — video canvas, brush/eraser strokes, GrabCut guess,
  Cutie subprocess. Depends on `app_core`.
- `track.py` — the Python that Cutie runs under. Lives at the lab
  workspace root, not in `draw_lab/`.

## Preview: proxy + async decoder — 2026-10-18

The old preview decoded 4K native, seeked to the nearest keyframe on
every scrub, and blocked the UI. It was three seconds per frame.
Replaced.

`app_core/src/video/proxy.rs` — `ensure(source)` returns the path to a
720p all-keyframe H.264 copy in `~/.cache/video_library/proxies/`.
Cache key is source path + mtime + size, so edits produce a fresh
proxy. Transcodes once on first open, 10–30s for a minute of 4K. Every
subsequent open is instant.

`app_core/src/video/decode_worker.rs` — a worker thread owns the
decoder (ffmpeg contexts aren't Send). The UI sends timestamps, the
worker decodes and sends frames back, and a generation counter drops
stale results. `VideoViewState::poll()` runs in the per-frame tick.
The playhead moves immediately on click; the frame arrives when it
arrives.

`VideoDecoder::open(path)` still takes the source path — it calls
`ensure` then opens the proxy. `open_proxy(source, proxy)` skips the
transcode. `path()` returns the source, `proxy_path()` returns the
proxy, `fps()` returns the source frame rate.

## draw_lab: draw → guess → track — 2026-10-18

`draw_lab` is where the tracking pipeline was proven. It's not wired
into the main app yet.

Drawing: strokes are normalized `[0,1]` points, stored per-frame in a
`HashMap<usize, StrokeList>` keyed on frame index. Scrub away from a
frame and its strokes disappear; scrub back and they return.
`Tool::Brush` and `Tool::Eraser` are separate stroke types; the brush
draws green, the eraser draws red, but the eraser's real job is
subtracting from the GrabCut mask, not from the visible strokes.
`Undo point` removes the tail of the last stroke (roughly a quarter,
minimum 3 points). `Undo stroke` removes the last stroke. `Clear`
removes everything. Brush size is a slider 1–60.

Guess: `draw_lab/src/guess.rs` runs OpenCV's `grabCut` on the current
frame. Brush points become foreground seeds, eraser points become
background. The result is saved as a binary PNG at
`<video_parent>/<video_stem>_tracking/seed_NNNNN.png`, where `NNNNN`
is the frame it was guessed on. `guess_frame` records that index.

Track: `run_track` spawns `.venv/bin/python track.py` with
`--video <proxy> --mask <seed> --out <masks_dir> --seed-frame N`.
`PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True` is set on the
environment. The child runs in the background, and `poll_tracking`
checks its exit status every frame. Output goes to
`<video_parent>/<video_stem>_tracking/masks/NNNNN.png`.

Review: the canvas overlays the current frame's mask in red. Play steps
at the source fps. A `_tracking/` folder already on disk is picked up
on Load — the mask count shows in the status line, and the red overlay
works without re-running Cutie.

Correcting: scrub to a bad frame, draw on the object, Guess, Track.
That overwrites the masks from `seed_frame` forward and leaves the
good ones before it intact. This is the "add more detail" loop from the
plan.

## Cutie setup — 2026-10-18

Cutie is the tracker. Python, PyTorch, GPU. It runs on this machine
specifically because the version pins are chosen for the GTX 1070
(Pascal, compute capability 6.1).

Installed at `/home/zakke/ProgStuff/egui/video_library_lab/`:

- `.venv/` — venv
- `Cutie/` — repo clone, `pip install -e .` done
- `Cutie/weights/` — cutie-base-mega.pth, coco_lvis_h18_itermask.pth

Versions that work (do not change casually):

- Python 3.13
- torch 2.6.0+cu124, torchvision 0.21.0+cu124, index-url
  `https://download.pytorch.org/whl/cu124`
- imageio, imageio-ffmpeg, pillow

Why those pins:

- Pascal support (SM_61) exists up to cu124 builds of torch 2.6. cu128
  builds of 2.8 dropped it. cu121 builds work but the driver refuses
  them ("driver too old, found 12040").
- torchvision 0.20 has no Python 3.13 wheel. 0.21 does.
- On CPU Cutie runs at ~1.3 s/frame, which is the same pain the
  preview fix was solving. GPU is not optional.

`track.py` specifics:

- Runs under `torch.inference_mode()` and `torch.autocast("cuda", ...)`.
  Without them, on an 8 GB card, it OOMs on the second frame — the
  graph is kept alive and full precision blows the budget.
- Resizes frames to 480p before feeding Cutie. `--max-height` to
  change.
- Caps the memory bank at 5 frames
  (`processor.memory_manager.max_mem_frames = 5`). Default is 10 and it
  OOMs.
- Seeds with `processor.step(image, mask_t, [1])` — 3D image tensor,
  2D mask tensor, list of object labels. Not 4D image, not bare int.
- Writes one PNG per frame plus a `done` sentinel.
- It does not need `.to(device)` on the processor; `get_default_model()`
  already puts the model on GPU. `InferenceCore` has no `.to` method.

## Kdenlive integration — 2026-10-18

D-Bus against D-Ogi's Kdenlive fork. The fork doesn't compile as
pushed — duplicate `Q_SCRIPTABLE` declarations in `src/mainwindow.h`
and a duplicated `Scripting API Implementation` block in
`src/mainwindow.cpp`. The user's local checkout at
`/home/zakke/kdenlive/` has those removed manually. Installed binary
at `/home/zakke/ProgStuff/kdenlive-fork/`. Launcher `~/.local/bin/kdn`
reads the install root from `~/.config/kdn/root`.

D-Bus interface (verified with `busctl introspect`):

- Service: `org.kde.kdenlive-<pid>` — NOT `org.kde.kdenlive.scripting`.
- Object: `/MainWindow`
- Interface: `org.kde.kdenlive.MainWindow`
- Methods used: `scriptGetSelection`, `scriptGetTimelineClipInfo`,
  `scriptGetProjectFps`, `scriptGetProjectPath`, `scriptImportMedia`,
  `scriptGetAllClipIds`, `scriptGetClipProperties`,
  `scriptDeleteTimelineClip`, `scriptInsertClip`,
  `scriptClearSelection`.

`scriptInsertClip` returns `-1` on this setup, always. Do not attempt
to fix from the client side. Accepted workflow: pull, edit, add to bin,
drag manually.

`scriptImportMedia` is async — returns `[]` even when the import
succeeded. Client polls `scriptGetAllClipIds` / `scriptGetClipProperties`
for the URL. Kdenlive also dedupes by file content, so re-importing
the same file returns `[]`.

`scriptGetClipProperties` returns an empty dict for phantom IDs, which
appear after failed timeline operations.

## Environment gotchas — 2026-10-18

- `c` wraps cargo and finds `local_cargo.sh`. `c b` builds, `c r`
  runs, `c c` checks. `local_cargo.sh` sets
  `RUSTFLAGS="-C link-arg=-fuse-ld=mold"` and disables the user's
  global cranelift setting. Cranelift is required off because `fontdue`
  (used in `app/src/bin/fading_popup.rs`) panics under cranelift with
  `llvm.x86.sse.cvtss2si is not yet supported`.
- zapcli is a file-change app, not a shell. Uses `create /abs/path` +
  `@new`, `replace /abs/path` + `@old` / `@new`, `overwrite /abs/path`
  + `@new`. Absolute paths only. Chains are all-or-nothing.
- The fullwidth asterisk `＊` (U+FF0A) appears in some yt-dlp filenames
  because the original YouTube title used it. Copy-pasting from `ls`
  output gives the right character. Typing `*` gives the wrong one.
- egui 0.36 changed `eframe::App` — the method is `fn ui(&mut self, ui:
  &mut egui::Ui, frame: &mut eframe::Frame)`, not `update(ctx, frame)`.
  `dropped_files` is `Vec<Arc<dyn DroppedFile>>`; use `f.path()` (which
  returns `&Path`, not `Option<&Path>`).
- `gen` is a reserved keyword in edition 2024. Rename to `generation`.
- `opencv` crate needs `clang-runtime` feature on Debian trixie with
  clang 19, and `imgproc` listed as an explicit feature.
  `default-features = false` disables every module. `core` is implicit,
  not a feature.

## Parked — 2026-10-18

- Two-clip aligner. Line up two clips by hand, A scales up just enough
  to cover B's frame, B eases back to native size. Renders to one file.
  Not started. Separate screen, doesn't share the tracker.
- Effects that hang off a track. Border, zoom toward it, B&W outside
  it, slow blur outside it. The track shape (position + size per frame)
  is what they'd read. Not started.
- Wiring draw_lab into the main app. Lab logic to `app_core`, UI to
  `every_page_ever`. Not started.
- "Applied X" badge on the loaded clip. Session-only, resets on new
  load. Not started.
- Main app rebuild against the new `app_core` — `video_view.rs` in the
  main app still uses the old `scrub_to` API. The lab proved the new
  one works; the main app needs the same change.

## Rules the user has set

From `~/ProgStuff/Workflow.txt` and this conversation:

- One step of a plan at a time.
- No code until asked. Talk the design through first.
- Short answers, prose not bullets.
- Take questions literally. "What is X" gets what X is, not what it
  does.
- No guessing. Say when unsure.
- Don't run cargo build unless it matters.
- Don't refactor code the user wrote.
- Never `unwrap()`. Errors carry `err_msg`, `method`, `filepath`.
- Visual → `every_page_ever`, logic → `app_core`. Folder per concern,
  file per thing.
- Popups wrap long text, don't truncate. Errors get a copy-last-error
  mechanism.
