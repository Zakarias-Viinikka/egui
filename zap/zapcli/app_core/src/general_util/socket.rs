use super::path_parsing::home_dir;
use std::io;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use zap_wire::{read_frame, write_frame, Frame, FrameKind};

const SOCKET: &str = "/run/user/1000/zap.sock";
const GUI: &str = "zapgui";

/// How long send() waits for any frame before giving up. This is the
/// guard against an out-of-date zapgui that reads the request the old way
/// (waiting for EOF that never comes) and never replies.
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

/// A connection with a request in flight. Frames arrive on a channel from a
/// reader thread, so the UI can later drain them per frame without blocking.
/// The writer half stays open so a cancel frame can go out while a run is
/// streaming.
pub struct Session {
    /// The writer half. None until a streaming request finishes connecting;
    /// connect + auto-launch happens on the reader thread so a slow launch
    /// never blocks the UI. `cancel_flag` covers a cancel issued before
    /// that.
    writer: Arc<Mutex<Option<UnixStream>>>,
    cancel_flag: Arc<AtomicBool>,
    frames: mpsc::Receiver<Frame>,
    _reader: thread::JoinHandle<()>,
}

impl Session {
    pub fn recv(&self) -> Option<Frame> {
        self.frames.recv().ok()
    }

    pub fn recv_timeout(&self, d: Duration) -> Result<Frame, mpsc::RecvTimeoutError> {
        self.frames.recv_timeout(d)
    }

    pub fn try_recv(&self) -> Option<Frame> {
        self.frames.try_recv().ok()
    }

    /// Ask the run to stop. If the connection isn't up yet, set a flag
    /// the reader thread checks the moment it connects.
    pub fn send_cancel(&mut self) -> Result<(), String> {
        let mut g = self
            .writer
            .lock()
            .map_err(|_| "zap: writer lock poisoned".to_string())?;
        match g.as_mut() {
            Some(w) => write_frame(w, &Frame::cancel())
                .map_err(|e| format!("zap: cancel write failed: {e}")),
            None => {
                self.cancel_flag.store(true, Ordering::SeqCst);
                Ok(())
            }
        }
    }
}

fn start_reader(
    mut stream: UnixStream,
    writer: Arc<Mutex<Option<UnixStream>>>,
    cancel_flag: Arc<AtomicBool>,
) -> mpsc::Receiver<Frame> {
    // Publish the writer clone before reading, so cancel can be sent while
    // the loop runs.
    if let Ok(clone) = stream.try_clone() {
        if let Ok(mut g) = writer.lock() {
            *g = Some(clone);
        }
    }
    if cancel_flag.load(Ordering::SeqCst) {
        let _ = write_frame(&mut stream, &Frame::cancel());
    }
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || loop {
        match read_frame(&mut stream) {
            Ok(f) => {
                if tx.send(f).is_err() {
                    break;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                let _ = tx.send(Frame::reply(format!(
                    "zap: bad frame ({e}); is zapgui up to date?"
                )));
                break;
            }
        }
    });
    rx
}

/// Open a connection and send `payload` as a Request frame. Returns a
/// Session whose reader thread is already pushing frames into a channel.
pub fn open(payload: &str) -> Result<Session, String> {
    let mut stream = connect_or_launch()?;

    write_frame(&mut stream, &Frame::request(payload))
        .map_err(|e| format!("zap: write failed: {e}"))?;

    let writer: Arc<Mutex<Option<UnixStream>>> = Arc::new(Mutex::new(None));
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let frames = start_reader(stream, writer.clone(), cancel_flag.clone());
    Ok(Session {
        writer,
        cancel_flag,
        frames,
        _reader: thread::spawn(|| {}),
    })
}

/// Open a connection and send a CargoRun frame. Returns a Session whose
/// reader thread is already pushing frames into a channel.
pub fn open_cargo(cwd: &std::path::Path, args: &[String]) -> Result<Session, String> {
    let frame = Frame::cargo_run(&cwd.display().to_string(), args)?;
    let writer: Arc<Mutex<Option<UnixStream>>> = Arc::new(Mutex::new(None));
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let writer_for_thread = writer.clone();
    let cancel_for_thread = cancel_flag.clone();

    let _reader = thread::spawn(move || {
        let mut stream = match connect_or_launch() {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Frame::reply(e));
                return;
            }
        };
        if let Err(e) = write_frame(&mut stream, &frame) {
            let _ = tx.send(Frame::reply(format!("zap: write failed: {e}")));
            return;
        }
        if let Ok(clone) = stream.try_clone() {
            if let Ok(mut g) = writer_for_thread.lock() {
                *g = Some(clone);
            }
        }
        if cancel_for_thread.load(Ordering::SeqCst) {
            let _ = write_frame(&mut stream, &Frame::cancel());
        }
        loop {
            match read_frame(&mut stream) {
                Ok(f) => {
                    if tx.send(f).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    let _ = tx.send(Frame::reply(format!(
                        "zap: bad frame ({e}); is zapgui up to date?"
                    )));
                    break;
                }
            }
        }
    });

    Ok(Session {
        writer,
        cancel_flag,
        frames: rx,
        _reader,
    })
}

/// Open a connection and send a RunFile frame. Same shape as open_cargo.
pub fn open_run(cwd: &std::path::Path, args: &[String]) -> Result<Session, String> {
    let frame = Frame::run_file(&cwd.display().to_string(), args)?;
    let writer: Arc<Mutex<Option<UnixStream>>> = Arc::new(Mutex::new(None));
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let writer_for_thread = writer.clone();
    let cancel_for_thread = cancel_flag.clone();

    let _reader = thread::spawn(move || {
        let mut stream = match connect_or_launch() {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Frame::reply(e));
                return;
            }
        };
        if let Err(e) = write_frame(&mut stream, &frame) {
            let _ = tx.send(Frame::reply(format!("zap: write failed: {e}")));
            return;
        }
        if let Ok(clone) = stream.try_clone() {
            if let Ok(mut g) = writer_for_thread.lock() {
                *g = Some(clone);
            }
        }
        if cancel_for_thread.load(Ordering::SeqCst) {
            let _ = write_frame(&mut stream, &Frame::cancel());
        }
        loop {
            match read_frame(&mut stream) {
                Ok(f) => {
                    if tx.send(f).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    let _ = tx.send(Frame::reply(format!(
                        "zap: bad frame ({e}); is zapgui up to date?"
                    )));
                    break;
                }
            }
        }
    });

    Ok(Session {
        writer,
        cancel_flag,
        frames: rx,
        _reader,
    })
}

/// Send a request and collect frames until the connection closes. Returns
/// the text of the last Reply frame, or an error if nothing arrived.
/// For a plain request this is exactly one Reply.
pub fn send(payload: &str) -> Result<String, String> {
    let session = open(payload)?;
    let mut reply = String::new();
    let deadline = Instant::now() + REPLY_TIMEOUT;
    loop {
        let remaining = match deadline.checked_duration_since(Instant::now()) {
            Some(d) if !d.is_zero() => d,
            _ => {
                return Err(
                    "zap: no reply from zapgui; is it up to date?".to_string()
                );
            }
        };
        match session.recv_timeout(remaining) {
            Ok(f) => {
                if f.kind == FrameKind::Reply {
                    reply = f.text;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(reply),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(
                    "zap: no reply from zapgui; is it up to date?".to_string()
                );
            }
        }
    }
}

fn connect_or_launch() -> Result<UnixStream, String> {
    match UnixStream::connect(SOCKET) {
        Ok(s) => Ok(s),
        Err(_) => {
            let gui = home_dir().join(".local/bin").join(GUI);
            Command::new(&gui)
                .arg("--hidden")
                .spawn()
                .map_err(|e| format!("zap: could not launch {}: {e}", gui.display()))?;
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if let Ok(conn) = UnixStream::connect(SOCKET) {
                    return Ok(conn);
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(format!("zap: could not connect to {SOCKET}"))
        }
    }
}
