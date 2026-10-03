use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::thread;

pub fn spawn(socket_path: &'static str) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let _ = std::fs::remove_file(socket_path);
        let listener = match UnixListener::bind(Path::new(socket_path)) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("terminal_router: bind failed: {}", e);
                return;
            }
        };
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("terminal_router: accept failed: {}", e);
                    continue;
                }
            };
            let mut buf = String::new();
            if stream.read_to_string(&mut buf).is_err() {
                continue;
            }
            // The socket is bidirectional. zapcli shuts down its write side
            // after sending, so read_to_string returns at EOF, and this write
            // reaches zapcli's still-open read side.
            let reply = crate::terminal_router::route_requests::route(buf.trim_end());
            let _ = stream.write_all(reply.as_bytes());
        }
    })
}
