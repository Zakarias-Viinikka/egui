//! Wire types shared by zapcli and zapgui: framed messages over a Unix
//! socket, plus the cargo allow-list that the cargo verb will consult.
//!
//! Frame layout on the wire:
//!
//! ```text
//! [kind: u8][length: u32 little-endian][bytes: length UTF-8]
//! ```
//! One connection carries one request. A plain request is: zapcli sends a
//! Request frame, zapgui replies with one Reply frame, then the connection
//! closes. Stream frames are carried on the same connection and described
//! by the plan; only the frame type lives here.

pub mod cargo_gate;
pub mod chain;
pub mod config;
pub mod hidden;
pub mod linefold;

use std::io::{self, Read, Write};

/// Maximum size of a single frame's payload, in bytes. A user paste of a
/// chain should fit comfortably; anything bigger is refused with a clear
/// error rather than allocating an unbounded buffer.
pub const MAX_FRAME_BYTES: u32 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// zapcli -> zapgui. Text is the request, same text as today.
    Request = 0x01,
    /// zapcli -> zapgui. Stop the current run. Text is empty.
    Cancel = 0x02,
    /// zapcli -> zapgui. Start a cargo run. Text is `<cwd>\0<arg1>\0...`.
    CargoRun = 0x03,
    /// zapcli -> zapgui. Run a file. Text is `<cwd>\0<arg1>\0...`.
    RunFile = 0x04,
    /// zapcli -> zapgui. A chain that ends in a run step. Text is
    /// `<cwd>\n<block>`. The connection stays open through review, the
    /// writes, and the run.
    ChainRun = 0x05,
    /// zapgui -> zapcli. The single final answer for a plain request.
    Reply = 0x10,
    /// zapgui -> zapcli. One line for the main scrollback.
    Notice = 0x11,
    /// zapgui -> zapcli. Open the stream pane. Text is the title.
    StreamOpen = 0x12,
    /// zapgui -> zapcli. Append output. Text is the chunk.
    StreamData = 0x13,
    /// zapgui -> zapcli. Clear the pane's buffer.
    StreamClear = 0x14,
    /// zapgui -> zapcli. Hide the pane. Text is empty.
    StreamClose = 0x15,
    /// zapgui -> zapcli. The run finished. Text is the exit status.
    StreamDone = 0x16,
}

impl FrameKind {
    pub fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0x01 => Self::Request,
            0x02 => Self::Cancel,
            0x03 => Self::CargoRun,
            0x04 => Self::RunFile,
            0x05 => Self::ChainRun,
            0x10 => Self::Reply,
            0x11 => Self::Notice,
            0x12 => Self::StreamOpen,
            0x13 => Self::StreamData,
            0x14 => Self::StreamClear,
            0x15 => Self::StreamClose,
            0x16 => Self::StreamDone,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub kind: FrameKind,
    pub text: String,
}

impl Frame {
    pub fn request(text: impl Into<String>) -> Self {
        Self { kind: FrameKind::Request, text: text.into() }
    }
    pub fn cancel() -> Self {
        Self { kind: FrameKind::Cancel, text: String::new() }
    }
    /// Encode a cargo run request. `cwd` first, then each argument.
    /// Arguments must not contain NUL; the parser refuses if any do.
    pub fn cargo_run(cwd: &str, args: &[String]) -> Result<Self, String> {
        let mut parts = vec![cwd.to_string()];
        parts.extend(args.iter().cloned());
        for p in &parts {
            if p.contains('\0') {
                return Err("cargo: argument contains NUL".to_string());
            }
        }
        Ok(Self { kind: FrameKind::CargoRun, text: parts.join("\0") })
    }
    /// Decode a cargo run request into (cwd, args).
    pub fn decode_cargo_run(text: &str) -> Option<(String, Vec<String>)> {
        decode_cwd_and_args(text)
    }
    /// Encode a run-file request. Same shape as cargo_run.
    /// Encode a chain-run request. Text is `<cwd>\n<block>`, the same
    /// payload shape as `request`.
    pub fn chain_run(payload: impl Into<String>) -> Self {
        Self { kind: FrameKind::ChainRun, text: payload.into() }
    }
    pub fn run_file(cwd: &str, args: &[String]) -> Result<Self, String> {
        let mut parts = vec![cwd.to_string()];
        parts.extend(args.iter().cloned());
        for p in &parts {
            if p.contains('\0') {
                return Err("run: argument contains NUL".to_string());
            }
        }
        Ok(Self { kind: FrameKind::RunFile, text: parts.join("\0") })
    }
    /// Decode a run-file request into (cwd, args).
    pub fn decode_run_file(text: &str) -> Option<(String, Vec<String>)> {
        decode_cwd_and_args(text)
    }
    pub fn reply(text: impl Into<String>) -> Self {
        Self { kind: FrameKind::Reply, text: text.into() }
    }
    pub fn notice(text: impl Into<String>) -> Self {
        Self { kind: FrameKind::Notice, text: text.into() }
    }
    pub fn stream_open(title: impl Into<String>) -> Self {
        Self { kind: FrameKind::StreamOpen, text: title.into() }
    }
    pub fn stream_data(text: impl Into<String>) -> Self {
        Self { kind: FrameKind::StreamData, text: text.into() }
    }
    pub fn stream_clear() -> Self {
        Self { kind: FrameKind::StreamClear, text: String::new() }
    }
    pub fn stream_close() -> Self {
        Self { kind: FrameKind::StreamClose, text: String::new() }
    }
    pub fn stream_done(status: impl Into<String>) -> Self {
        Self { kind: FrameKind::StreamDone, text: status.into() }
    }
}

fn decode_cwd_and_args(text: &str) -> Option<(String, Vec<String>)> {
    let mut parts = text.split('\0').map(|s| s.to_string());
    let cwd = parts.next()?;
    let args: Vec<String> = parts.collect();
    Some((cwd, args))
}

pub fn write_frame<W: Write>(w: &mut W, frame: &Frame) -> io::Result<()> {
    let bytes = frame.text.as_bytes();
    if bytes.len() as u64 > MAX_FRAME_BYTES as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("frame payload {} bytes exceeds max {}", bytes.len(), MAX_FRAME_BYTES),
        ));
    }
    let mut header = [0u8; 5];
    header[0] = frame.kind as u8;
    header[1..5].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    w.write_all(&header)?;
    w.write_all(bytes)?;
    w.flush()?;
    Ok(())
}

pub fn read_frame<R: Read>(r: &mut R) -> io::Result<Frame> {
    let mut header = [0u8; 5];
    r.read_exact(&mut header)?;
    let kind = FrameKind::from_byte(header[0]).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown frame kind 0x{:02x}", header[0]),
        )
    })?;
    let len = u32::from_le_bytes([header[1], header[2], header[3], header[4]]);
    if len > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("frame length {len} exceeds max {MAX_FRAME_BYTES}"),
        ));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    let text = String::from_utf8(buf).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("invalid utf-8: {e}"))
    })?;
    Ok(Frame { kind, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_kind() {
        let frames = [
            Frame::request("hello"),
            Frame::cancel(),
            Frame::chain_run("payload"),
            Frame::reply("world"),
            Frame::notice("one line"),
            Frame::stream_open("cargo run"),
            Frame::stream_data("Compiling ...\n"),
            Frame::stream_clear(),
            Frame::stream_close(),
            Frame::stream_done("exit 1"),
        ];
        let mut buf: Vec<u8> = Vec::new();
        for f in &frames {
            write_frame(&mut buf, f).expect("write");
        }
        let mut cursor = std::io::Cursor::new(buf);
        for expected in &frames {
            let got = read_frame(&mut cursor).expect("read");
            assert_eq!(&got, expected);
        }
    }

    #[test]
    fn rejects_unknown_kind() {
        let bytes = [0xFFu8, 0, 0, 0, 0];
        let mut cursor = std::io::Cursor::new(bytes);
        let err = read_frame(&mut cursor).expect_err("should fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_huge_length() {
        // kind = Reply, length = u32::MAX
        let bytes = [0x10u8, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut cursor = std::io::Cursor::new(bytes);
        let err = read_frame(&mut cursor).expect_err("should fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn empty_payload_is_ok() {
        let mut buf: Vec<u8> = Vec::new();
        write_frame(&mut buf, &Frame::reply("")).expect("write");
        let mut cursor = std::io::Cursor::new(buf);
        let got = read_frame(&mut cursor).expect("read");
        assert_eq!(got.kind, FrameKind::Reply);
        assert_eq!(got.text, "");
    }
}


pub mod argv {
    //! Split a cargo command line into arguments. Single quotes, double
    //! quotes and backslash are honoured; nothing else is special, so $,
    //! backticks, pipes and globs are plain text. Never calls a shell.

    pub fn split(line: &str) -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        let mut current = String::new();
        let mut chars = line.chars().peekable();
        let mut in_single = false;
        let mut in_double = false;
        let mut has_current = false;
        while let Some(c) = chars.next() {
            if in_single {
                if c == '\'' {
                    in_single = false;
                } else {
                    current.push(c);
                }
                continue;
            }
            if in_double {
                if c == '"' {
                    in_double = false;
                } else if c == '\\' {
                    if let Some(&next) = chars.peek() {
                        chars.next();
                        current.push(next);
                    }
                } else {
                    current.push(c);
                }
                continue;
            }
            match c {
                '\'' => { in_single = true; has_current = true; }
                '"' => { in_double = true; has_current = true; }
                '\\' => {
                    if let Some(&next) = chars.peek() {
                        chars.next();
                        current.push(next);
                        has_current = true;
                    }
                }
                c if c.is_whitespace() => {
                    if has_current {
                        out.push(std::mem::take(&mut current));
                        has_current = false;
                    }
                }
                c => { current.push(c); has_current = true; }
            }
        }
        if in_single {
            return Err("unterminated single quote".to_string());
        }
        if in_double {
            return Err("unterminated double quote".to_string());
        }
        if has_current {
            out.push(current);
        }
        Ok(out)
    }

    #[cfg(test)]
    mod tests {
        use super::split;

        #[test]
        fn plain_words() {
            assert_eq!(split("build --release").unwrap(), vec!["build", "--release"]);
        }

        #[test]
        fn double_quotes_keep_spaces() {
            assert_eq!(split("run \"a b\"").unwrap(), vec!["run", "a b"]);
        }

        #[test]
        fn single_quotes_keep_dollar_literal() {
            assert_eq!(split("run '$HOME'").unwrap(), vec!["run", "$HOME"]);
        }

        #[test]
        fn backslash_escapes_space() {
            assert_eq!(split("run a\\ b").unwrap(), vec!["run", "a b"]);
        }

        #[test]
        fn unterminated_quotes_error() {
            assert!(split("run 'x").is_err());
            assert!(split("run \"x").is_err());
        }

        #[test]
        fn empty_gives_empty() {
            assert_eq!(split("").unwrap(), Vec::<String>::new());
            assert_eq!(split("   ").unwrap(), Vec::<String>::new());
        }
    }
}
