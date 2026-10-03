use super::super::{Line, Outcome, Session};
use crate::general_util::path_parsing::resolve;
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 20;

pub fn run(args: &[&str], session: &Session) -> Outcome {
    let joined = args.join(" ");
    let paths: Vec<&str> = joined
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if paths.is_empty() {
        return Outcome::Print(vec![Line::error("cat: missing file")]);
    }

    let mut copy = String::new();
    let mut copied: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for (i, raw) in paths.iter().enumerate() {
        let resolved = resolve(raw, &session.cwd);
        if i > 0 {
            copy.push('\n');
        }
        copy.push_str(&format!("=== {} ===\n", resolved.display()));

        match std::fs::read_to_string(&resolved) {
            Ok(content) => {
                copy.push_str(&content);
                if !content.ends_with('\n') {
                    copy.push('\n');
                }
                copied.push((*raw).to_string());
            }
            Err(_) => {
                copy.push_str("failed copy.\n");
                if let Some(parent) = nearest_existing(&resolved) {
                    copy.push_str(&format!("nearest existing parent: {}\n", parent.display()));
                    for name in list_entries(&parent, MAX_ENTRIES) {
                        copy.push_str(&format!("  {name}\n"));
                    }
                } else {
                    copy.push_str("no existing parent found.\n");
                }
                failed.push((*raw).to_string());
            }
        }
    }

    let summary = if failed.is_empty() {
        format!("copied {} to clipboard", copied.join(", "))
    } else if copied.is_empty() {
        format!("failed: {} (nothing copied)", failed.join(", "))
    } else {
        format!(
            "copied {} to clipboard (failed: {})",
            copied.join(", "),
            failed.join(", ")
        )
    };

    Outcome::PrintAndCopy {
        lines: vec![Line::cli(summary)],
        text: copy,
    }
}

fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut probe = path.parent().map(|p| p.to_path_buf());
    while let Some(dir) = probe {
        if dir.exists() {
            return Some(dir);
        }
        probe = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn list_entries(dir: &Path, max: usize) -> Vec<String> {
    let Ok(iter) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = iter
        .flatten()
        .take(max)
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                format!("{name}/")
            } else {
                name
            }
        })
        .collect();
    names.sort();
    names
}
