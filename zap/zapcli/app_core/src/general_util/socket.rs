use super::path_parsing::home_dir;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const SOCKET: &str = "/run/user/1000/zap.sock";
const GUI: &str = "zapgui";

/// Send a request to zapgui and return its reply.
///
/// If zapgui isn't running, launch it and retry connecting for a few seconds.
/// After sending, shut down the write half so zapgui's read_to_string returns
/// at EOF, then read the reply from the still-open read half.
pub fn send(payload: &str) -> Result<String, String> {
    let stream = match UnixStream::connect(SOCKET) {
        Ok(s) => Some(s),
        Err(_) => {
            let gui = home_dir().join(".local/bin").join(GUI);
            if let Err(e) = Command::new(&gui).spawn() {
                return Err(format!("zap: could not launch {}: {e}", gui.display()));
            }
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut s = None;
            while Instant::now() < deadline {
                if let Ok(conn) = UnixStream::connect(SOCKET) {
                    s = Some(conn);
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            s
        }
    };

    let mut stream = match stream {
        Some(s) => s,
        None => return Err(format!("zap: could not connect to {SOCKET}")),
    };

    if let Err(e) = stream.write_all(payload.as_bytes()) {
        return Err(format!("zap: write failed: {e}"));
    }
    let _ = stream.shutdown(Shutdown::Write);

    let mut reply = String::new();
    if let Err(e) = stream.read_to_string(&mut reply) {
        return Err(format!("zap: read failed: {e}"));
    }
    Ok(reply)
}
