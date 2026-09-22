use std::sync::mpsc::{channel, Receiver};

use eframe::egui;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};

const CHANNEL: &str = "zakkeakke";

pub fn spawn_reader(ctx: egui::Context) -> Receiver<(String, String)> {
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
