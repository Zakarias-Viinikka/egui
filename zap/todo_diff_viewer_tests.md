# todo — diff viewer regression tests

Future work. Not now.

## What I want

Regression tests for the diff viewer. I don't want to manage them
myself. They should exist so that if the viewer starts doing things
differently than it's supposed to, something catches it automatically.

The two areas I most want covered are the move display and
edit-vs-replace, because those are the places where the viewer is
allowed to "lie" for readability, and a regression there would be easy
to miss by eye.

## Shape of the tests

The idea is a dump: turn the viewer's render tree into plain text, one
line per row, describing everything the viewer draws that matters for
regression. Then the test compares that text against an expected text.
The dump is only ever seen by the test comparison, not by me.

What the dump should include per row:
- the kind of the row (equal, change, delete, insert)
- the bar color on the left side and on the right side
- the text on the left side and on the right side
- any move label strip, including "(edited)"
- whether the row is part of a grayed-out moved block

What the dump should NOT include:
- word wrap and continuation markers (that's a display convenience for
  me, not something to regress-test)
- the syntax color of individual letters

One dump per test case: left side against right side of the shown
diff. The changes-only / full-file toggle is a display choice and is
not part of the dump.

## Where the tests live

Doesn't matter to me. The diff engine already has unit tests inside
pair.rs and moves.rs, so those are a natural home, or a tests/ folder.

## After the tests: unmonolith

A separate pass after the tests exist. The zap workspace is a
monolith right now and I want it broken into small focused crates,
each with one job.

A good example of the structure I want is:
/home/zakke/ProgStuff/rust_practice/server_template_3

Small crates under client_side/, server_side/, shared/, each doing one
thing, with clear names. The shared/error_stuff and shared/
server_protocol pattern (shared code in its own small crate) is the
kind of thing I want.

## Order

Tests first, against the current layout. Then the unmonolith as its
own pass. Don't combine them; I want the tests passing on the current
code before anything gets moved.

## For the smart AI

Turn this into a preplan, then a numbered plan of steps. The
regression tests and the unmonolith are separate steps. Read the
diff viewer and diff engine first (see CONTEXT_FOR_AI.md for where
they live). Ask about anything unclear before planning.
