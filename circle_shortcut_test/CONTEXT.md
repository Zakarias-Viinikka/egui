# circle_shortcut_test — context

This is a throwaway test project. Its only job is to figure out how
"ctrl + shortcut" should behave in zutil, without the noise of the real app.

## The real thing this is for

zutil is a personal knowledge tool. Its main UI is a circle menu.

At the root you see a handful of nav items: Templates, AI Prompts, Text,
Terminal Commands, Projects. Click one, and the circle is replaced with the
next set of items — categories, then the things inside a category, then
finally something you can act on.

Every item in a circle can have a user-assigned shortcut. You right-click
the little box to the left of an item and pick one. Shortcuts fire only
while the circle that owns them is on screen. So a shortcut set on "git"
under "Templates" only works once you've already navigated into Templates.

## The problem

Getting to a deep thing takes clicks. From the root, to run a specific git
command you might go: Templates -> git -> view diff. Three clicks.

You want one keystroke for that. Not to start the chain, but to end it —
press ctrl+g and the app should just run "view diff", with the menu staying
where it is.

## What ctrl+shortcut should do

When you press ctrl+X, the app looks up shortcut X on the current circle.
If that item is a nav item, it looks up X again on the circle that item
would take you to, and keeps going. When it lands on an item that does
something (copy text, open a view), it just does that thing, and does NOT
move the menu.

One press. Silent resolution. The menu stays where it was.

Contrast: plain X fires the item on the current circle only.

## What went wrong in the first attempt

The first attempt (in zutil, before this test project existed) tried to do
this by repeatedly dispatching shortcuts and checking whether the menu
changed. That was wrong on two counts:

1. It actually pushed each level onto the menu stack, so the user saw the
   menu hop through levels. The user wants the menu to stay put.
2. It could loop. Nothing stopped it from firing the same combo over and
   over if the chain was circular.

The fix in this test project: walk the chain by reading the tree directly,
never calling `push`. Only call `fire` once, at the end, on the item that
does something.

## The tree in this test project

    root
      Templates  [g]  -> templates
      Text       [t]  -> text

    templates
      git        [g]  -> templates/git
      adb        [a]  -> templates/adb

    text
      notes      [n]  -> text/notes

    templates/git
      view diff  [g]  does "view diff"
      commit     [c]  does "commit"

    templates/adb
      devices    [v]  does "list devices"

    text/notes
      first      [1]  does "view note 1"
      second     [2]  does "view note 2"

Every circle is hardcoded. No DB. No zutil code.

## How to test

    cd ~/ProgStuff/egui/circle_shortcut_test
    ./local_cargo.sh

The window shows a circle with the current circle's items, a "<" button
top left when you're deeper than root, right-click to pop, and a HUD at
the bottom that says which circle you're on, the stack depth, and the
last action that fired.

Try:

- Press `g` on root. Menu navs to templates, then press `g` again. Menu
  navs to templates/git, then press `g` again. "view diff" fires, HUD
  says `do "view diff"`, menu stays on templates/git.
- Restart. Press ctrl+g on root. It should NOT nav. It should just fire
  "view diff" and stay on root.
- Restart. Press ctrl+c on root. Root has no `c`, so nothing happens.
- Nav to templates. Press ctrl+g. Fires "view diff", stays on templates.
- Nav to templates. Press ctrl+a. Navs to adb (via a, no ctrl).
  Press ctrl+v. Fires "list devices".

## Success

ctrl+<combo> walks the shortcut chain from the current circle silently and
fires the terminal action, without touching the menu stack. If it can't
find the combo anywhere in the chain, it does nothing.

## After this test works

Port the working logic into zutil. Specifically:

- `app.rs`, in the block that reads fresh key presses for shortcuts.
- Delete the old "descend" loop that pushes levels.
- Replace with: walk the current menu stack's `MenuItem`s, using their
  `kind` field to know whether an item is a Push (nav) or an action, and
  follow shortcuts without pushing.
- The zutil equivalent of "find the same combo in the next circle" is
  to look at `crate::globals::current_items()` after a hypothetical push,
  which means reading the same data the push would produce. Cleanest
  way: add a method on App like `peek_nav(&self, item) -> Vec<MenuItem>`
  that computes what the next circle would contain without mutating the
  stack, then walk that.

## Unrelated but worth knowing

- Build with `./local_cargo.sh`, never bare `cargo`. Global cargo config
  forces the cranelift backend which panics on float-to-int instructions.
- The user's style: plain answers, no bullet spam, one thing at a time,
  no code until asked. See their workflow doc.
