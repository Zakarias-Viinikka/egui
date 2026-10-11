# todo — later

- report::build has no length cap; long chains make a long report.
- popup position and wording are first guesses; tune once popups work.
- rm can't take a path with a space: the line splits on whitespace, so
  `rm a b` is two paths and `DELETE REQUEST,` becomes two tokens.
- rename the rm verb to delete. `rm` typed by accident in a terminal is
  dangerous.
- rework `s`. Today it opens settings; it should do what `s` does in the
  bashrc.
- add `cdf`, defined in the bashrc.
- word wrap on the input line in zapcli is wrong. Pasting wraps but
  continuation lines start indented; typing runs past the right edge.
- `last` should give diff plus current contents, and copy to clipboard
  by default.
- a rename command.
- the edit_title pen button should work: turn the block into a text
  field, Enter commits.
- file extensions should be an explicit allowlist.
- every allowed format needs an explicit statement of what each verb
  does to it.
- a chain that mixes auto-apply-safe steps with one rm/mv should only
  ask for approval on the rm/mv step, not the whole chain. Today any
  rm or mv forces the entire block into review, so a single delete on
  an otherwise whitelisted edit makes you look at every step.
- a chmod step for chains. `chmod +x <path>` as a one-line step, so an
  AI can make a script runnable without leaving zap. It should always
  go to review, like rm and mv, and the review window should show a
  distinct banner (something like "EXEC REQUEST"), since granting the
  executable bit is a security-relevant change. The whole point is the
  human gets a prompt to allow a file to be made executable before it
  happens.

- replace failures should be smart. When a replace's old text isn't found,
  instead of just saying "expected 1 occurrence, found 0", the report
  should try to locate the nearest matching lines in the file and include
  them in the clipboard sentence. Like: "expected 1 occurrence of
  `fn foo() {`, found 0. Closest match at line 47: `fn foo() -> bool {`."
  The similarity metric already exists in text_diff/similarity.rs; reuse
  it. The goal is that a paste-back to the AI says "here's what you did
  wrong" without a follow-up cat. Same treatment for create-target-exists
  and overwrite-file-missing if a useful hint is cheap. A "smart system"
  that figures out the mistake from similarity, not a generic error.
