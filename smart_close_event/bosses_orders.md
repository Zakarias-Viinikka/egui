# bosses_orders

the goal: history is a Vec of pages, top one is what's shown.
each page owns a modal, modals chain into a linked list for nesting.
going back closes a modal until there's none left, then pops the page.
back on the first page with nothing open does nothing.

right now there's no history at all. one page, one slot in an Arc<Mutex>.
no vec, no push, no pop, no modals.

HistoryManager lives in main. HistoryManager::new returns a tx.
the tx gets passed to the egui drawer struct.
the drawer struct clones the tx down into each page enum.
pages send messages over the tx to push/pop pages and modals.

i need to pass the Arc<Mutex<PageToRouteTo>> down to the actual page
draw function, so the page can change which page is shown.

i need to make a few pages and have buttons that change page or open a
modal or whatever.

then i need to make it so the pages are given a tx thing so they can talk
to the struct that owns the history and say "add this to history" or
"push/remove a modal on the current page".