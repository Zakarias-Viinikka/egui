use std::sync::mpsc::Sender;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, GrabMode, KeyButMask, ModMask};
use x11rb::protocol::Event;

const XK_DELETE: u32 = 0xFFFF;
const XK_INSERT: u32 = 0xFF63;

pub enum Cmd {
    ToggleHistory,
    ToggleHidden,
    Quit,
}

pub fn spawn_hotkey_thread(tx: Sender<Cmd>) {
    std::thread::spawn(move || {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("hotkey: connect failed: {e}");
                return;
            }
        };
        let setup = conn.setup();
        let root = setup.roots[screen_num].root;
        let min = setup.min_keycode;
        let max = setup.max_keycode;
        let count = max - min + 1;
        let cookie = match conn.get_keyboard_mapping(min, count) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("hotkey: get_keyboard_mapping failed: {e}");
                return;
            }
        };
        let mapping = match cookie.reply() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("hotkey: keyboard mapping failed: {e}");
                return;
            }
        };
        let per = mapping.keysyms_per_keycode as usize;

        let find = |sym: u32| -> Option<u8> {
            for (i, chunk) in mapping.keysyms.chunks(per).enumerate() {
                if chunk.iter().any(|&k| k == sym) {
                    return Some(min + i as u8);
                }
            }
            None
        };

        let del_kc = match find(XK_DELETE) {
            Some(k) => k,
            None => {
                eprintln!("hotkey: Delete keycode not found");
                return;
            }
        };
        let ins_kc = match find(XK_INSERT) {
            Some(k) => k,
            None => {
                eprintln!("hotkey: Insert keycode not found");
                return;
            }
        };

        let super_only = [
            ModMask::M4,
            ModMask::M4 | ModMask::M2,
            ModMask::M4 | ModMask::LOCK,
            ModMask::M4 | ModMask::M2 | ModMask::LOCK,
        ];
        let super_alt = [
            ModMask::M4 | ModMask::M1,
            ModMask::M4 | ModMask::M1 | ModMask::M2,
            ModMask::M4 | ModMask::M1 | ModMask::LOCK,
            ModMask::M4 | ModMask::M1 | ModMask::M2 | ModMask::LOCK,
        ];

        for m in super_only {
            if let Err(e) = conn.grab_key(true, root, m, del_kc, GrabMode::ASYNC, GrabMode::ASYNC) {
                eprintln!("hotkey: grab Super+Delete failed: {e}");
                return;
            }
            if let Err(e) = conn.grab_key(true, root, m, ins_kc, GrabMode::ASYNC, GrabMode::ASYNC) {
                eprintln!("hotkey: grab Super+Insert failed: {e}");
                return;
            }
        }
        for m in super_alt {
            if let Err(e) = conn.grab_key(true, root, m, del_kc, GrabMode::ASYNC, GrabMode::ASYNC) {
                eprintln!("hotkey: grab Super+Alt+Delete failed: {e}");
                return;
            }
        }
        if let Err(e) = conn.flush() {
            eprintln!("hotkey: flush failed: {e}");
            return;
        }
        loop {
            match conn.wait_for_event() {
                Ok(Event::KeyPress(ev)) => {
                    let cmd = if ev.detail == del_kc {
                        if ev.state.contains(KeyButMask::MOD1) {
                            Cmd::ToggleHidden
                        } else {
                            Cmd::Quit
                        }
                    } else if ev.detail == ins_kc {
                        Cmd::ToggleHistory
                    } else {
                        continue;
                    };
                    if tx.send(cmd).is_err() {
                        return;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("hotkey: event error: {e}");
                    return;
                }
            }
        }
    });
}
