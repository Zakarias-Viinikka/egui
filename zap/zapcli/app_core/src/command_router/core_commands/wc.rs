use super::super::{Line, Outcome, Session};
use crate::general_util::path_parsing::resolve;

const BINARY_PROBE_BYTES: usize = 8 * 1024;

pub fn run(args: &[&str], session: &Session) -> Outcome {
    let mut want_lines = false;
    let mut want_words = false;
    let mut want_bytes = false;
    let mut files: Vec<String> = Vec::new();
    let mut any_flag = false;
    for a in args {
        if *a == "-l" {
            want_lines = true;
            any_flag = true;
            continue;
        }
        if *a == "-w" {
            want_words = true;
            any_flag = true;
            continue;
        }
        if *a == "-c" {
            want_bytes = true;
            any_flag = true;
            continue;
        }
        if a.len() > 1 && a.starts_with('-') {
            return Outcome::Print(vec![Line::error(format!("wc: unsupported option {a}"))]);
        }
        files.push(a.to_string());
    }
    if !any_flag {
        want_lines = true;
        want_words = true;
        want_bytes = true;
    }
    if files.is_empty() {
        return Outcome::Print(vec![Line::error("wc: missing file")]);
    }

    let mut rows: Vec<(usize, usize, usize, String)> = Vec::new();
    let mut total = (0usize, 0usize, 0usize);
    for raw in &files {
        let path = resolve(raw, &session.cwd);
        if let Some(msg) = zap_wire::hidden::HiddenSet::check(&path) {
            return Outcome::Print(vec![Line::error(msg)]);
        }
        match wc_one(&path) {
            Ok((l, w, c)) => {
                total.0 += l;
                total.1 += w;
                total.2 += c;
                rows.push((l, w, c, path.display().to_string()));
            }
            Err(e) => {
                return Outcome::Print(vec![Line::error(e)]);
            }
        }
    }
    let multi = files.len() > 1;
    if multi {
        rows.push((total.0, total.1, total.2, "total".to_string()));
    }

    let widths = wc_widths(&rows, want_lines, want_words, want_bytes);
    let mut out: Vec<Line> = Vec::new();
    for (l, w, c, name) in &rows {
        out.push(Line::peek(format_wc(
            *l, *w, *c, name, want_lines, want_words, want_bytes, &widths,
        )));
    }
    Outcome::Peek(out)
}

fn wc_one(path: &std::path::Path) -> Result<(usize, usize, usize), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("wc: {}: {e}", path.display()))?;
    if meta.is_dir() {
        return Err(format!("wc: {}: is a directory", path.display()));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("wc: {}: {e}", path.display()))?;
    let probe_end = bytes.len().min(BINARY_PROBE_BYTES);
    if bytes[..probe_end].contains(&0u8) {
        return Err(format!("wc: {}: binary file", path.display()));
    }
    let lines = bytes.iter().filter(|b| **b == b'\n').count();
    let text = String::from_utf8_lossy(&bytes);
    let words = text.split_whitespace().count();
    Ok((lines, words, bytes.len()))
}

struct Widths {
    lines: usize,
    words: usize,
    bytes: usize,
}

fn wc_widths(rows: &[(usize, usize, usize, String)], wl: bool, ww: bool, wc: bool) -> Widths {
    let mut w = Widths { lines: 0, words: 0, bytes: 0 };
    for (l, wo, c, _) in rows {
        if wl {
            w.lines = w.lines.max(l.to_string().len());
        }
        if ww {
            w.words = w.words.max(wo.to_string().len());
        }
        if wc {
            w.bytes = w.bytes.max(c.to_string().len());
        }
    }
    w
}

fn format_wc(
    lines: usize,
    words: usize,
    bytes: usize,
    name: &str,
    wl: bool,
    ww: bool,
    wc: bool,
    widths: &Widths,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if wl {
        parts.push(format!("{:>width$}", lines, width = widths.lines));
    }
    if ww {
        parts.push(format!("{:>width$}", words, width = widths.words));
    }
    if wc {
        parts.push(format!("{:>width$}", bytes, width = widths.bytes));
    }
    parts.push(name.to_string());
    parts.join(" ")
}
