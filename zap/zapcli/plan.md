# safet plan

## in safet now
- ls
- cd
- cat
- s / settings

## commands to add next
- head
- tail
- find  (rg under the hood)
- grep  (rg under the hood)
- rg
- wc
- git read-only: status, log, diff, show

## my custom bashrc commands, sorted

### safe as-is (read-only or clipboard only)
- cur — pwd to clipboard
- copycat — cat + clipboard
- t — tree + clipboard (drop the `clear`; safet has no terminal to clear)

### were fzf pickers, need a safet version
- cdf — pick a folder in cwd
- s — pick a project dir
- b — go up N dirs, or pick from a list
- co — copy a filename
- cop — copy a full path
- run — pick and run a script (works differently; details TBD)

### spawns an external process, decide
- zed — open a file in the editor
- settings — already in, same shape

### needs a guardrail or a hard no
- c — cargo wrapper; cargo builds code, and build scripts run arbitrary code
- tw — start stream
- windows — reboot
- mp — open in mousepad (you said no)

## open questions
- what "run" means in safet if it's not an fzf picker
- fzf pickers in safet: a GUI list, or something else
- c: allow but warn, allow with an approval step, or block
- which external process launches are fine (zed yes, what else)

## whitelisting (new)

Two whitelists, different purposes.

### executable whitelist
A list of executables safet is allowed to run. Anything not on the list is
refused. This is what makes it safe to put command-running commands into
safet at all — the command only gets to invoke binaries that are on the
list.

Rule that goes with it: an executable on the list is whitelisted for what
it does *now*, not for what it might do after someone edits it. So if the
binary's file changes, the whitelist entry stops trusting it until the
change is verified. Otherwise "whitelisted executable" would mean "any
future version of this file is trusted", which defeats the whole point.

### edit whitelist (file / folder)
A list of files or folders where the AI is allowed to make edits without
you approving each one. Everything else goes through the normal
review-then-approve flow. This is the "I trust this area, let it move
fast" escape hatch.

### open questions
- where the whitelists live (a file in the project? in ~/.config/safet/?)
- what "verified" means for the executable whitelist — hash the binary,
  ask you once per change, or something else
- can an edit whitelist entry be a whole project, or only a subfolder
