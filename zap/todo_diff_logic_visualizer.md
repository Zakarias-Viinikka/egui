# todo — diff logic visualizer

Future work. Not now.

## What I want

A standalone egui program, in the zap crate, that is only a visual
tool. Its one job is to help me and the AI figure out the rule for
"is this a change (edit), or is this a delete plus an insert".

The point is not to ship anything. The point is to tune the rule
visually, in one place, so I can look at a diff and say "yes, that's
right" or "no, this should be different". No more guessing at
thresholds in my head.

## Shape

Buttons across the top. Each button is one test case: a named before
and after pair. Clicking a button shows that pair's diff with the
current rule applied.

Below the buttons, the diff renders like the review window's viewer
does. Same row kinds, same colors, same bars. Left and right side by
side.

That's it. No writing files, no chain, no review window. Just: pick a
case, look at the result.

## Cases I care about

Things near the boundary, where the rule could go either way:

- a line with a comma added
- a line with a variable renamed
- a line reindented
- a line where the whole body changed but the signature stayed
- a line where two unrelated things happen to share a token or two
- a small block that got one line edited and the rest stayed

I'll add more cases as I find them while using it.

## Where the rule lives

The rule is in text_diff/similarity.rs (SIMILARITY_THRESHOLD) and the
pairing logic in text_diff/pair.rs. The visualizer should call the
same code those do, so what I see in the tool is what the review
window will show.

## After the rule is tuned

Once I'm happy with the rule, the visualizer stays as a tool I can
reopen when I want to check a case. It is not thrown away.

## For the smart AI

Turn this into a preplan, then a numbered plan of steps when the time
comes. Read the diff engine first (text_diff/ in CONTEXT_FOR_AI.md).
Ask about anything unclear before planning. Keep it small; the whole
point is that it's a tool, not a product.
