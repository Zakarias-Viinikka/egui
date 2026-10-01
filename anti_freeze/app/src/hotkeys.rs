use std::sync::mpsc::Sender;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, GrabMode, ModMask};
use x11rb::protocol::Event;

const XK_PAGE_UP: u32 = 0xFF55;

pub enum Cmd {
    ToggleVisible,
    RamHigh,
    CapOver,
}

pub fn spawn_hotkey_thread(tx: Sender<Cmd>, ctx: eframe::egui::Context) {
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

        let mut page_up_kc: Option<u8> = None;
        for (i, chunk) in mapping.keysyms.chunks(per).enumerate() {
            if chunk.iter().any(|&k| k == XK_PAGE_UP) {
                page_up_kc = Some(min + i as u8);
                break;
            }
        }
        let page_up_kc = match page_up_kc {
            Some(k) => k,
            None => {
                eprintln!("hotkey: Page_Up keycode not found");
                return;
            }
        };

        let masks = [
            ModMask::M4,
            ModMask::M4 | ModMask::M2,
            ModMask::M4 | ModMask::LOCK,
            ModMask::M4 | ModMask::M2 | ModMask::LOCK,
        ];
        for m in masks {
            if let Err(e) =
                conn.grab_key(true, root, m, page_up_kc, GrabMode::ASYNC, GrabMode::ASYNC)
            {
                eprintln!("hotkey: grab Super+Page_Up failed: {e}");
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
                    if ev.detail == page_up_kc {
                        if tx.send(Cmd::ToggleVisible).is_err() {
                            return;
                        }
                        ctx.request_repaint();
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
