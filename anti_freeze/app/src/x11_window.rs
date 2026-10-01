use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode};
use x11rb::wrapper::ConnectionExt as _;

pub fn set_skip_taskbar() {
    std::thread::spawn(|| {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("x11_window: connect failed: {e}");
                return;
            }
        };
        let root = conn.setup().roots[screen_num].root;

        let net_wm_name = match conn.intern_atom(false, b"_NET_WM_NAME") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("x11_window: intern _NET_WM_NAME reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("x11_window: intern _NET_WM_NAME failed: {e}");
                return;
            }
        };
        let utf8_string = match conn.intern_atom(false, b"UTF8_STRING") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("x11_window: intern UTF8_STRING reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("x11_window: intern UTF8_STRING failed: {e}");
                return;
            }
        };
        let net_wm_state = match conn.intern_atom(false, b"_NET_WM_STATE") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("x11_window: intern _NET_WM_STATE reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("x11_window: intern _NET_WM_STATE failed: {e}");
                return;
            }
        };
        let skip_taskbar = match conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("x11_window: intern SKIP_TASKBAR reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("x11_window: intern SKIP_TASKBAR failed: {e}");
                return;
            }
        };
        let skip_pager = match conn.intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("x11_window: intern SKIP_PAGER reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("x11_window: intern SKIP_PAGER failed: {e}");
                return;
            }
        };

        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let tree_cookie = match conn.query_tree(root) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("x11_window: query_tree failed: {e}");
                    return;
                }
            };
            let tree = match tree_cookie.reply() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("x11_window: query_tree reply failed: {e}");
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
                if name != "anti_freeze" {
                    continue;
                }
                if let Err(e) = conn.change_property32(
                    PropMode::REPLACE,
                    win,
                    net_wm_state,
                    AtomEnum::ATOM,
                    &[skip_taskbar, skip_pager],
                ) {
                    eprintln!("x11_window: set SKIP_TASKBAR failed: {e}");
                    return;
                }
                if let Err(e) = conn.flush() {
                    eprintln!("x11_window: flush failed: {e}");
                }
                return;
            }
        }
        eprintln!("x11_window: window not found");
    });
}

pub fn raise_self() {
    std::thread::spawn(|| {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("x11_window: raise connect failed: {e}");
                return;
            }
        };
        let root = conn.setup().roots[screen_num].root;

        let net_wm_name = match conn.intern_atom(false, b"_NET_WM_NAME") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("x11_window: raise intern reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("x11_window: raise intern failed: {e}");
                return;
            }
        };
        let utf8_string = match conn.intern_atom(false, b"UTF8_STRING") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => {
                    eprintln!("x11_window: raise intern reply failed: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("x11_window: raise intern failed: {e}");
                return;
            }
        };

        let tree_cookie = match conn.query_tree(root) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("x11_window: raise query_tree failed: {e}");
                return;
            }
        };
        let tree = match tree_cookie.reply() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("x11_window: raise query_tree reply failed: {e}");
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
            if name != "anti_freeze" {
                continue;
            }
            let _ = conn.configure_window(
                win,
                &x11rb::protocol::xproto::ConfigureWindowAux::new()
                    .stack_mode(x11rb::protocol::xproto::StackMode::ABOVE),
            );
            let _ = conn.flush();
            return;
        }
        eprintln!("x11_window: raise: window not found");
    });
}
