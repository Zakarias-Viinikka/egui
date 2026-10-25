use super::super::{Line, Outcome, Session};
use crate::general_util::path_parsing::resolve;

const MAX_OUTPUT_LINES: usize = 2000;
const MAX_LINE_CHARS: usize = 4000;
const BINARY_PROBE_BYTES: usize = 8 * 1024;
const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;

pub fn run(args: &[&str], session: &Session) -> Outcome {
    let (count, files) = match parse_args(args, 10) {
        Ok(p) => p,
        Err(e) => return Outcome::Print(vec![Line::error(e)]),
    };
    if files.is_empty() {
        return Outcome::Print(vec![Line::error("tail: missing file")]);
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
        match tail_one(&path, count) {
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
    let text: String = out
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Outcome::PeekAndCopy { lines: out, text }
}

fn parse_args(args: &[&str], default: usize) -> Result<(usize, Vec<String>), String> {
    let mut count = default;
    let mut files: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        if a == "-n" {
            if i + 1 >= args.len() {
                return Err("tail: -n needs a number".to_string());
            }
            count = args[i + 1].parse().map_err(|_| format!("tail: bad number: {}", args[i + 1]))?;
            i += 2;
            continue;
        }
        if let Some(rest) = a.strip_prefix("-n") {
            if !rest.is_empty() {
                count = rest.parse().map_err(|_| format!("tail: bad number: {rest}"))?;
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
            return Err(format!("tail: unsupported option {a}"));
        }
        files.push(a.to_string());
        i += 1;
    }
    Ok((count, files))
}

fn tail_one(path: &std::path::Path, count: usize) -> Result<Vec<String>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("tail: {}: {e}", path.display()))?;
    if meta.is_dir() {
        return Err(format!("tail: {}: is a directory", path.display()));
    }
    if meta.len() > MAX_READ_BYTES {
        return Err(format!("tail: {}: file too big ({} bytes)", path.display(), meta.len()));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("tail: {}: {e}", path.display()))?;
    let probe_end = bytes.len().min(BINARY_PROBE_BYTES);
    if bytes[..probe_end].contains(&0u8) {
        return Err(format!("tail: {}: binary file", path.display()));
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let skip = lines.len().saturating_sub(count);
    let mut out: Vec<String> = Vec::new();
    for l in &lines[skip..] {
        let trimmed = l.strip_suffix('\n').unwrap_or(l);
        out.push(truncate_line(trimmed));
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
