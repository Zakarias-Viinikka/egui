# todo - immediate

## ls, should auto copy to clipboard.

##i want a button that shows me the new version of the file, but without all the gaps. so it still has the edit/replace/delete markers, but it doesnt have all the gaps. it doesn't need to match left side anymore, like left side can just be whatever. if there's a smart way to make the left side still "fit" or something that's fine. or if not, then just dont

##mv requests are shown as "rename requests" for some reason. like on right side on the buttons it says mv. but on the actual requests it says rename. im pretty sure its suppoed to be just move

##["zap's delete is text files only, and refuses directories. rmdir needs empty. There's no recursive delete in zap, so target/ (which has hundreds of nested files) can't be removed from here.

This is the one step that has to happen in your normal terminal:"]

##make the special request less shiny. i dont like when stuff just glows unecessarily

##retry made me manually "allow" even thought it was whitelisted. if it wasnt retry and it worked without it, it wouldn't have

##error messages on big fails should mention "use retry if possible. either to delete steps that failed or to redo them" or something. because ai keeps rewriting entire thing when it fails

##clicking in the whitelist menu should close it

##chmod loks like "execute request" it's vizually confuising and the "special request hting" is too glowy anyway tone it down

##whenever i run a zap that edits. zed is stale. i need to either fix that, or update my zed fork so it has support so it runs a web socket i can talk to. so that i can tell it "refresh x" if a page it has open is edited.

##would be nice if repasting same thing, would be a popup that was like "you arleady pasted this" but only for commands that are long enough. sometimes i forget if i ran something or not

## [ai keeps messing this up: o you not know how retry works? nothing has been aplied from the big chain that failed.

U can either run retry and rmeove the ones that failed. or u can retry and fixe them and it will rerun the entire thing and replace the ones that failed with your fixes
The user is right. I was overthinking. Nothing from the big chain applied. So the current files are exactly what they were before the big chain — the states shown in the pasted text ARE the pre-chain states.

So retry with the 5 broken slots replaced with corrected bodies will re-run the entire 36-step chain, using the original bodies for the 31 working slots and my fixed bodies for slots 17, 20, 32, 33, 34.

Now I need to fix the 5 failing patterns based on what the files actually look like:

Step 17 (controls.rs): My pattern was:] the error message on big fails need to describe clearly what i explained to ai "o you not know how retry works? nothing has been aplied from the big chain that failed.

U can either run retry and rmeove the ones that failed. or u can retry and fixe them and it will rerun the entire thing and replace the ones that failed with your fixes"]

## add rg support so this works for example "rg -n 'server_link|kick_send|identity|two_tab\b|start_server_link' /home/zakke/ProgStuff/ZakkeN/cqrs_state_synchronization/web_interface/src"

also make error messages do "couldn't find x, suggesting using zap rg command" so ai doesnt spend time thinkign hen i can just show it stuff with rg.