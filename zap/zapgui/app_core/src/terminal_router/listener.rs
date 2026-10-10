use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;
use zap_wire::{read_frame, write_frame, Frame, FrameKind};

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
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("terminal_router: accept failed: {}", e);
                    continue;
                }
            };
            thread::spawn(move || handle_one(stream));
        }
    })
}

fn handle_one(mut stream: UnixStream) {
    let frame = match read_frame(&mut stream) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return,
        Err(e) => {
            let _ = write_frame(
                &mut stream,
                &Frame::reply(format!(
                    "zapgui: bad frame ({e}); is zapcli up to date?"
                )),
            );
            return;
        }
    };

    if frame.kind == FrameKind::CargoRun {
        crate::terminal_router::route_requests::handle_cargo_run(&mut stream, &frame.text);
        return;
    }
    if frame.kind == FrameKind::RunFile {
        crate::terminal_router::route_requests::handle_run_file(&mut stream, &frame.text);
        return;
    }

    if frame.kind != FrameKind::Request {
        let _ = write_frame(
            &mut stream,
            &Frame::reply(format!(
                "zapgui: expected a request frame, got {:?}",
                frame.kind
            )),
        );
        return;
    }

    let request = frame.text.trim_end().to_string();
    if crate::terminal_router::route_requests::try_stream(&request, &mut stream) {
        return;
    }
    let reply = crate::terminal_router::route_requests::route(&request);
    let _ = write_frame(&mut stream, &Frame::reply(reply));
}
