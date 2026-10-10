# Bug: overwrite with absolute path ignores markers

`overwrite <absolute path>` always takes the old bare overwrite form, even
when `@new` / `@end` markers follow it. The markers get written into the
file as literal text instead of being parsed as delimiters.

The bare form is supposed to be "absolute path, no markers." In practice,
the parser sees the absolute path and commits to bare form before it ever
looks for markers. That means the marker form of overwrite is unusable
with an absolute path.

Workaround: use `~/...` or a relative path. Those take the marker form
correctly.

Expected: `overwrite` should decide which form it is by the presence or
absence of markers, not by the shape of the path. An absolute path plus
markers should be the chain form.
