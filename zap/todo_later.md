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