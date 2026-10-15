# zap_orchestrator — pre-plan

Decided 2026-10-15. Not started. This is the shape, not the spec — the
orchestration loop itself is specced in gemini_orchestrator_plan.md.

## What it is

A new crate in ~/ProgStuff/egui/zap that runs in the background, drives
the Gemini API, and executes the AI's chains through zap without the user
in the loop. It takes over the job the user does by hand today: feeding
the AI's edits into zap, and feeding zap's results back to the AI.

## Where it sits

It is a peer of zapcli, not a child of it. zapcli is a front-end for a
person — its output is prose shaped for reading. The orchestrator talks
to the zapgui socket directly through zap_wire, the same framing crate
zapcli uses. It never runs zapcli and scrapes its text output.

## Origin tagging

Frames get a field saying who sent them: Human or Orchestrator. zapcli
hardcodes Human; the orchestrator hardcodes Orchestrator. zapgui replies
differently depending on that — prose for a person, a machine shape for
the orchestrator. The machine shape is what the orchestrator feeds back
to Gemini, so it has to be structured, not a sentence.

## The connection

The orchestrator sends a chain and waits on the same connection until the
user has acted. That wait can be minutes or hours, so it cannot use
zapcli's 10-second REPLY_TIMEOUT, and it cannot block the rest of the
orchestrator while it waits. It drains stream frames as they come, so
progress reaches it live rather than at the end.

## The popup

A second binary inside the orchestrator crate. When a chain needs review,
the popup appears in the top-left of the screen and stays until clicked.
Clicking hands the chain to zapgui and zapgui's normal review window
opens. The popup holds the payload; zapgui knows nothing about it until
the click. It is a normal egui window, not the reusable_components popup
widget, which is a modal drawn inside an already-open window.

## The loop

Read the plan and files, pack them into one payload, append history, wait
on the rate limiter, call Gemini, extract the reply, append it to
history, decide whether the task is done. If the AI's reply contains a
chain, send it to zapgui and wait. When the result comes back, feed it to
Gemini and go round again. See gemini_orchestrator_plan.md for the details
of packaging, history and rate limiting.

## Decided

- Direct socket connection, through zap_wire.
- Origin tag is a plain enum, hardcoded per client.
- The connection stays open for the life of a chain.
- The popup lives in the orchestrator crate and hands off on click.

## Not decided — ask, do not assume

- What triggers a run. gemini_orchestrator_plan.md mentions a watched
  folder and a push; neither is chosen.
- Where the Gemini API key comes from.
- Whether the orchestrator may request edits outside whitelisted folders,
  or only inside them. This is the same question the safety gate is about.

## The safety gate

IMPORTANT_TODO.md in the crate states that the orchestrator must not be
given autonomous control of the zap interface until zap's safety has been
verified. It is a hard gate, not a footnote. What "verified" means is
undecided and the file should leave the answer blank rather than invent
one.

## Not in scope

- No changes to zapcli's human-facing behaviour.
- No changes to chain parsing or how edits are applied.
- No web client, no Android app.

## Files

~~~
/home/zakke/ProgStuff/egui/zap/gemini_orchestrator_plan.md
/home/zakke/ProgStuff/egui/zap/local_cargo.sh
/home/zakke/ProgStuff/egui/zap/zap_wire/src/lib.rs
/home/zakke/ProgStuff/egui/zap/zap_wire/src/chain.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/general_util/socket.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/command_router/mod.rs
/home/zakke/ProgStuff/egui/zap/zapcli/app_core/src/command_router/core_commands/chain.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/terminal_router/route_requests.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/terminal_router/retry.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/chain/mod.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/chain/report.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/chain/step.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/clipboard.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app_core/src/lib.rs
/home/zakke/ProgStuff/egui/zap/zapgui/app/src/main.rs
/home/zakke/ProgStuff/egui/zap/RETRY.md
/home/zakke/ProgStuff/egui/zap/CONTEXT_FOR_AI.md
/home/zakke/ProgStuff/egui/reusable_components/popup/src/ui.rs
~~~

## Prompt for the smart AI

~~~
You are adding a new crate, zap_orchestrator, to an existing Rust
workspace at /home/zakke/ProgStuff/egui/zap. Read
/home/zakke/ProgStuff/egui/zap/gemini_orchestrator_plan.md first — that is
the spec for the orchestration loop itself. This prompt is only about how
the orchestrator attaches to the existing zap tooling.

Context: zapcli is a terminal-like app a human types into. It parses
"chains" (blocks of file edits) and sends them over a Unix socket to
zapgui, which shows a diff review window the human confirms or denies.
Both sides share framing and parsing code in the zap_wire crate.
zapcli's socket client is in app_core/src/general_util/socket.rs.
zapgui's request router is in app_core/src/terminal_router/route_requests.rs.
Frames are defined in zap_wire/src/lib.rs. Chains are parsed in
zap_wire/src/chain.rs.

The orchestrator is a background process that drives the Gemini API and
executes the AI's chains through zap. It is a peer of zapcli, not a
wrapper around it: it must connect to the zapgui socket directly using
zap_wire, never by running zapcli and scraping its output.

Four things to build.

One: an origin tag on frames. Add a field to the frame type saying who
sent it — Human or Orchestrator. zapcli hardcodes Human; the orchestrator
hardcodes Orchestrator. zapgui replies in prose for a human and in a
structured machine-readable shape for the orchestrator. Decide the
machine shape and say why; it gets fed straight back to an LLM, so it
must be unambiguous and cheap to parse.

Two: a long-lived connection. The orchestrator sends a chain and stays
connected until the human has confirmed or denied, which may take hours.
It cannot use zapcli's REPLY_TIMEOUT of 10 seconds, and it must not block
its own event loop while waiting. It drains stream frames as they arrive
so progress reaches it live. Design the concurrency: the orchestrator has
to keep talking to Gemini while a chain sits waiting for review.

Three: a popup binary inside the orchestrator crate. When a chain needs
review, a small always-on-top window appears in the top-left of the
screen and stays until clicked. On click it hands the chain to zapgui,
which opens its normal review window. The popup holds the payload; zapgui
learns nothing until the click.
/home/zakke/ProgStuff/egui/reusable_components/popup/src/ui.rs is an
existing egui popup widget you can read as reference, but it is a modal
drawn inside an already-open window, not a standalone binary — you are
building something different. A normal egui window is fine; it does not
need to match that widget's behaviour.

Four: IMPORTANT_TODO.md in the new crate, stating that the orchestrator
must not be given autonomous control of the zap interface until zap's
safety has been verified. This is a hard gate, not a note. What
"verified" means is undecided — write the file so the question is
explicit, and leave the answer blank rather than inventing one.

Decisions already made, do not relitigate: the orchestrator connects to
the socket directly; the origin tag is a plain enum hardcoded per client;
the connection stays open for the life of a chain; the popup lives in the
orchestrator crate and hands off on click.

Decisions NOT made — ask before assuming: what triggers a run (the plan
doc mentions a watched folder and a push, but neither is chosen); where
the Gemini API key comes from; and whether the orchestrator may request
edits outside whitelisted folders or only inside them.

Do not change zapcli's human-facing behaviour. Do not change chain
parsing or how edits are applied. Do not touch the web client or the
Android app.

Read the existing code before designing. If the code and this prompt
disagree about how something currently works, the code wins — say so.
~~~
