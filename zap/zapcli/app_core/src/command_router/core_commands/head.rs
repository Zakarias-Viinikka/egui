use super::super::{Line, Outcome, Session};
use crate::general_util::path_parsing::resolve;
use std::io::BufRead;

const MAX_OUTPUT_LINES: usize = 2000;
const MAX_LINE_CHARS: usize = 4000;
const BINARY_PROBE_BYTES: usize = 8 * 1024;

pub fn run(args: &[&str], session: &Session) -> Outcome {
    let (count, files) = match parse_args(args, 10) {
        Ok(p) => p,
        Err(e) => return Outcome::Print(vec![Line::error(e)]),
    };
    if files.is_empty() {
        return Outcome::Print(vec![Line::error("head: missing file")]);
    }
    let mut out: Vec<Line> = Vec::new();
    let multi = files.len() > 1;
    for (i, raw) in files.iter().enumerate() {
        let path = resolve(raw, &session.cwd);
        if let Some(msg) = zap_wire::hidden::HiddenSet::check(&path) {
            out.push(Line::error(msg));
            continue;
        }
        if multi {
            if i > 0 {
                out.push(Line::peek(""));
            }
            out.push(Line::peek(format!("==> {} <==", path.display())));
        }
        match head_one(&path, count) {
            Ok(lines) => {
                for l in lines {
                    out.push(Line::peek(l));
                }
            }
            Err(e) => {
                out.push(Line::error(e));
            }
        }
        if out.len() > MAX_OUTPUT_LINES {
            out.truncate(MAX_OUTPUT_LINES);
            out.push(Line::peek(format!("... output truncated at {MAX_OUTPUT_LINES} lines")));
            break;
        }
    }
    Outcome::Peek(out)
}

fn parse_args(args: &[&str], default: usize) -> Result<(usize, Vec<String>), String> {
    let mut count = default;
    let mut files: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        if a == "-n" {
            if i + 1 >= args.len() {
                return Err("head: -n needs a number".to_string());
            }
            count = args[i + 1].parse().map_err(|_| format!("head: bad number: {}", args[i + 1]))?;
            i += 2;
            continue;
        }
        if let Some(rest) = a.strip_prefix("-n") {
            if !rest.is_empty() {
                count = rest.parse().map_err(|_| format!("head: bad number: {rest}"))?;
                i += 1;
                continue;
            }
        }
        if a.len() > 1 && a.starts_with('-') {
            if let Ok(n) = a[1..].parse::<usize>() {
                count = n;
                i += 1;
                continue;
            }
            return Err(format!("head: unsupported option {a}"));
        }
        files.push(a.to_string());
        i += 1;
    }
    Ok((count, files))
}

fn head_one(path: &std::path::Path, count: usize) -> Result<Vec<String>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("head: {}: {e}", path.display()))?;
    if meta.is_dir() {
        return Err(format!("head: {}: is a directory", path.display()));
    }
    let file = std::fs::File::open(path).map_err(|e| format!("head: {}: {e}", path.display()))?;
    let mut reader = std::io::BufReader::new(file);
    let mut first: Vec<u8> = Vec::with_capacity(BINARY_PROBE_BYTES);
    let mut probe_buf = [0u8; 4096];
    while first.len() < BINARY_PROBE_BYTES {
        let n = std::io::Read::read(&mut reader, &mut probe_buf)
            .map_err(|e| format!("head: {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        first.extend_from_slice(&probe_buf[..n]);
    }
    if first.contains(&0u8) {
        return Err(format!("head: {}: binary file", path.display()));
    }
    let file = std::fs::File::open(path).map_err(|e| format!("head: {}: {e}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let mut out: Vec<String> = Vec::new();
    for line in reader.lines().take(count) {
        let line = line.map_err(|e| format!("head: {}: {e}", path.display()))?;
        out.push(truncate_line(&line));
    }
    Ok(out)
}

fn truncate_line(s: &str) -> String {
    if s.chars().count() <= MAX_LINE_CHARS {
        return s.to_string();
    }
    let mut out: String = s.chars().take(MAX_LINE_CHARS).collect();
    out.push_str("...");
    out
}
