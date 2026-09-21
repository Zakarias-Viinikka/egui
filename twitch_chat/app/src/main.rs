use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, GrabMode, KeyButMask, ModMask, PropMode};
use x11rb::wrapper::ConnectionExt as _;
use x11rb::protocol::Event;

const CHANNEL: &str = "zakkeakke";
const FADE_AFTER: Duration = Duration::from_secs(15);
const FADE_DURATION: Duration = Duration::from_secs(2);
const VISIBLE_MAX: usize = 10;
const MAX_BOX_WIDTH: f32 = 400.0;

const XK_DELETE: u32 = 0xFFFF;
const XK_INSERT: u32 = 0xFF63;

const USER_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 140, 255);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 230, 230);

enum Cmd {
    ToggleHistory,
    ToggleHidden,
    Quit,
}

struct Message {
    user: String,
    text: String,
    arrived: Instant,
}

fn main() -> eframe::Result<()> {
    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    spawn_hotkey_thread(cmd_tx);
    spawn_shadow_killer();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 600.0])
            .with_position([10.0, 10.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_window_type(egui::X11WindowType::Dock)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "chat",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(cmd_rx)))),
    )
}

struct App {
    rx: Option<Receiver<(String, String)>>,
    cmds: Receiver<Cmd>,
    messages: Vec<Message>,
    show_history: bool,
    hotkey_hidden: bool,
}

impl App {
    fn new(cmds: Receiver<Cmd>) -> Self {
        Self {
            rx: None,
            cmds,
            messages: Vec::new(),
            show_history: false,
            hotkey_hidden: false,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint();

        let mut visuals = ui.style().visuals.clone();
        visuals.window_fill = egui::Color32::TRANSPARENT;
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.popup_shadow = egui::epaint::Shadow::NONE;
        ui.ctx().set_visuals(visuals);

        if self.rx.is_none() {
            self.rx = Some(spawn_reader(ui.ctx().clone()));
        }

        if let Some(rx) = &self.rx {
            while let Ok((user, text)) = rx.try_recv() {
                self.messages.push(Message {
                    user,
                    text,
                    arrived: Instant::now(),
                });
            }
        }

        while let Ok(cmd) = self.cmds.try_recv() {
            match cmd {
                Cmd::ToggleHistory => self.show_history = !self.show_history,
                Cmd::ToggleHidden => self.hotkey_hidden = !self.hotkey_hidden,
                Cmd::Quit => std::process::exit(0),
            }
        }

        let pointer_over = ui.ctx().input(|i| i.pointer.hover_pos().is_some());
        let content_visible = !self.hotkey_hidden && !pointer_over;

        if !content_visible {
            return;
        }

        if self.show_history {
            self.draw_history(ui);
        } else {
            self.draw_live(ui);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

impl App {
    fn draw_live(&self, ui: &mut egui::Ui) {
        let now = Instant::now();

        let visible: Vec<&Message> = self
            .messages
            .iter()
            .filter(|m| now.duration_since(m.arrived) < FADE_AFTER + FADE_DURATION)
            .collect();

        let start = visible.len().saturating_sub(VISIBLE_MAX);
        let visible = &visible[start..];

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 230))
                    .shadow(egui::epaint::Shadow::NONE)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_max_width(MAX_BOX_WIDTH - 16.0);
                        for m in visible {
                            let age = now.duration_since(m.arrived);
                            let alpha = if age <= FADE_AFTER {
                                1.0
                            } else {
                                let t = (age - FADE_AFTER).as_secs_f32()
                                    / FADE_DURATION.as_secs_f32();
                                (1.0 - t).clamp(0.0, 1.0)
                            };
                            let a = (alpha * 255.0) as u8;
                            let user_color = egui::Color32::from_rgba_unmultiplied(
                                USER_COLOR.r(),
                                USER_COLOR.g(),
                                USER_COLOR.b(),
                                a,
                            );
                            let text_color = egui::Color32::from_rgba_unmultiplied(
                                TEXT_COLOR.r(),
                                TEXT_COLOR.g(),
                                TEXT_COLOR.b(),
                                a,
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(&m.user).color(user_color).size(18.0));
                                ui.label(egui::RichText::new(&m.text).color(text_color).size(18.0));
                            });
                        }
                    });
            });
    }

    fn draw_history(&self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_black_alpha(220))
                    .shadow(egui::epaint::Shadow::NONE)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_max_width(MAX_BOX_WIDTH - 16.0);
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for m in self.messages.iter() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&m.user).color(USER_COLOR).size(18.0));
                                    ui.label(egui::RichText::new(&m.text).color(TEXT_COLOR).size(18.0));
                                });
                            }
                        });
                    });
            });
    }
}

fn spawn_reader(ctx: egui::Context) -> Receiver<(String, String)> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("twitch_chat: failed to start tokio runtime: {e}");
                return;
            }
        };
        rt.block_on(async move {
            let config = ClientConfig::default();
            let (mut incoming, client) =
                TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);
            if let Err(e) = client.join(CHANNEL.to_owned()) {
                eprintln!("twitch_chat: failed to join #{CHANNEL}: {e}");
                return;
            }
            while let Some(msg) = incoming.recv().await {
                if let ServerMessage::Privmsg(m) = msg {
                    if tx.send((m.sender.name, m.message_text)).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            }
        });
    });
    rx
}

fn spawn_hotkey_thread(tx: Sender<Cmd>) {
    std::thread::spawn(move || {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => { eprintln!("hotkey: connect failed: {e}"); return; }
        };
        let setup = conn.setup();
        let root = setup.roots[screen_num].root;
        let min = setup.min_keycode;
        let max = setup.max_keycode;
        let count = max - min + 1;
        let cookie = match conn.get_keyboard_mapping(min, count) {
            Ok(c) => c,
            Err(e) => { eprintln!("hotkey: get_keyboard_mapping failed: {e}"); return; }
        };
        let mapping = match cookie.reply() {
            Ok(m) => m,
            Err(e) => { eprintln!("hotkey: keyboard mapping failed: {e}"); return; }
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
            None => { eprintln!("hotkey: Delete keycode not found"); return; }
        };
        let ins_kc = match find(XK_INSERT) {
            Some(k) => k,
            None => { eprintln!("hotkey: Insert keycode not found"); return; }
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
                Err(e) => { eprintln!("hotkey: event error: {e}"); return; }
            }
        }
    });
}



fn spawn_shadow_killer() {
    std::thread::spawn(|| {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => { eprintln!("shadow: connect failed: {e}"); return; }
        };
        let root = conn.setup().roots[screen_num].root;

        let gtk_frame_extents = match conn.intern_atom(false, b"_GTK_FRAME_EXTENTS") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => { eprintln!("shadow: intern atom reply failed: {e}"); return; }
            },
            Err(e) => { eprintln!("shadow: intern atom failed: {e}"); return; }
        };
        let net_wm_name = match conn.intern_atom(false, b"_NET_WM_NAME") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => { eprintln!("shadow: intern atom reply failed: {e}"); return; }
            },
            Err(e) => { eprintln!("shadow: intern atom failed: {e}"); return; }
        };
        let utf8_string = match conn.intern_atom(false, b"UTF8_STRING") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => { eprintln!("shadow: intern atom reply failed: {e}"); return; }
            },
            Err(e) => { eprintln!("shadow: intern atom failed: {e}"); return; }
        };
        let net_wm_state = match conn.intern_atom(false, b"_NET_WM_STATE") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => { eprintln!("shadow: intern atom reply failed: {e}"); return; }
            },
            Err(e) => { eprintln!("shadow: intern atom failed: {e}"); return; }
        };
        let net_wm_state_above = match conn.intern_atom(false, b"_NET_WM_STATE_ABOVE") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(e) => { eprintln!("shadow: intern atom reply failed: {e}"); return; }
            },
            Err(e) => { eprintln!("shadow: intern atom failed: {e}"); return; }
        };

        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(200));

            let tree_cookie = match conn.query_tree(root) {
                Ok(c) => c,
                Err(e) => { eprintln!("shadow: query_tree failed: {e}"); return; }
            };
            let tree = match tree_cookie.reply() {
                Ok(t) => t,
                Err(e) => { eprintln!("shadow: query_tree reply failed: {e}"); return; }
            };

            for &win in &tree.children {
                let prop_cookie = match conn.get_property(false, win, net_wm_name, utf8_string, 0, 1024) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let prop = match prop_cookie.reply() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let name = String::from_utf8_lossy(&prop.value).to_string();
                if name == "chat" {
                    // Remove dock shadow
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
                    // Force always-on-top
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
                    if let Err(e) = conn.flush() {
                        eprintln!("shadow: flush failed: {e}");
                    }
                    eprintln!("shadow: set extents and above on {name}");
                    return;
                }
            }
        }
        eprintln!("shadow: window not found");
    });
}
