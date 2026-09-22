//! X11 glue that turns the chat window into a click-through overlay.
//!
//! eframe's `with_mouse_passthrough(true)` does nothing on X11 — winit's
//! `set_cursor_hittest` is a no-op there. So we do it ourselves on the X
//! side.
//!
//! The function polls the root window's children for up to ten seconds,
//! looking for a window whose `_NET_WM_NAME` is `"chat"`. When found, it
//! runs this sequence in one burst, no sleeps:
//!
//! 1. `_GTK_FRAME_EXTENTS` zeroed, so the compositor stops drawing a shadow.
//! 2. `_NET_WM_STATE_ABOVE` set, so the window stays on top.
//! 3. `override_redirect` enabled — the WM stops managing the window.
//! 4. Empty XFixes input region — X routes mouse events below the window.
//! 5. Reparent to root.
//!
//! Testing notes — what was actually verified:
//!
//! - Steps 1-5 together work consistently on this machine's X11 setup
//!   (real X server, not XWayland).
//! - Removing step 5 (reparent) and rebuilding made the window start
//!   stealing clicks again. Adding it back made it work again. So step 5
//!   is load-bearing even though steps 3 and 4 should already have made
//!   the window unmanaged and input-transparent on their own.
//! - Adding a sleep between step 3 and step 4 (first 200ms, then 1s)
//!   caused failure on most runs. Removing the sleep restored consistent
//!   success. This was observed, not explained.
//! - Steps 1-4 were not tested individually. Only the full sequence, the
//!   reparent-removed variant, and the sleep-added variant were tested.
//!
//! Things that were tried and did not work in isolation:
//!
//! - `shape_rectangles(SO::SET, SK::INPUT, ClipOrdering::UNSORTED, ..., &[])`
//!   on the chat window. The call succeeded and querying it back showed the
//!   input region was empty, but clicks still landed on the window.
//! - `xfixes_set_window_shape_region` on the *parent* frame window. The call
//!   succeeded, but querying it back immediately showed the WM had restored
//!   the full input region. The WM treats its frame as its own and reverts
//!   external shape changes on it.
//! - `with_mouse_passthrough(true)` in the eframe `ViewportBuilder`. No
//!   effect — winit doesn't implement this on X11.
//!
//! Open questions: why does the ordering matter, why does adding a sleep
//! hurt, and whether this will survive a WM restart or upgrade. None of
//! these are understood.
//!
use x11rb::connection::Connection;
use x11rb::protocol::shape::SK;
use x11rb::protocol::xfixes::{ConnectionExt as _, RegionWrapper};
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode};
use x11rb::wrapper::ConnectionExt as _;

pub fn apply() {
    std::thread::spawn(|| {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("shadow: connect failed: {e}");
                return;
            }
        };
        let root = conn.setup().roots[screen_num].root;

        let gtk_frame_extents = match conn.intern_atom(false, b"_GTK_FRAME_EXTENTS") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("shadow: intern atom reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("shadow: intern atom failed: {e}");
                return;
            }
        };
        let net_wm_name = match conn.intern_atom(false, b"_NET_WM_NAME") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("shadow: intern atom reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("shadow: intern atom failed: {e}");
                return;
            }
        };
        let utf8_string = match conn.intern_atom(false, b"UTF8_STRING") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("shadow: intern atom reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("shadow: intern atom failed: {e}");
                return;
            }
        };
        let net_wm_state = match conn.intern_atom(false, b"_NET_WM_STATE") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("shadow: intern atom reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("shadow: intern atom failed: {e}");
                return;
            }
        };
        let net_wm_state_above = match conn.intern_atom(false, b"_NET_WM_STATE_ABOVE") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("shadow: intern atom reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("shadow: intern atom failed: {e}");
                return;
            }
        };

        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));

            let tree_cookie = match conn.query_tree(root) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("shadow: query_tree failed: {e}");
                    return;
                }
            };
            let tree = match tree_cookie.reply() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("shadow: query_tree reply failed: {e}");
                    return;
                }
            };

            for &win in &tree.children {
                let prop_cookie =
                    match conn.get_property(false, win, net_wm_name, utf8_string, 0, 1024) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                let prop = match prop_cookie.reply() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let name = String::from_utf8_lossy(&prop.value).to_string();
                if name != "chat" {
                    continue;
                }

                if let Err(e) = conn.change_property32(
                    PropMode::REPLACE,
                    win,
                    gtk_frame_extents,
                    AtomEnum::CARDINAL,
                    &[0u32, 0, 0, 0],
                ) {
                    eprintln!("shadow: set extents failed: {e}");
                    return;
                }

                if let Err(e) = conn.change_property32(
                    PropMode::REPLACE,
                    win,
                    net_wm_state,
                    AtomEnum::ATOM,
                    &[net_wm_state_above],
                ) {
                    eprintln!("shadow: set above failed: {e}");
                    return;
                }

                match conn.change_window_attributes(
                    win,
                    &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
                        .override_redirect(1),
                ) {
                    Ok(_) => eprintln!("shadow: set override_redirect on chat"),
                    Err(e) => eprintln!("shadow: set override_redirect failed: {e}"),
                }

                let empty_region = match RegionWrapper::create_region(&conn, &[]) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("shadow: create empty region failed: {e}");
                        return;
                    }
                };
                if let Err(e) =
                    conn.xfixes_set_window_shape_region(win, SK::INPUT, 0, 0, empty_region.region())
                {
                    eprintln!("shadow: set input shape failed: {e}");
                    return;
                }

                if let Err(e) = conn.reparent_window(win, root, 10, 10) {
                    eprintln!("shadow: reparent failed: {e}");
                } else if let Err(e) = conn.flush() {
                    eprintln!("shadow: flush failed: {e}");
                } else {
                    eprintln!("shadow: reparented to root {}", root);
                }

                eprintln!("shadow: set extents and above on {name}");
                return;
            }
        }
        eprintln!("shadow: window not found");
    });
}
