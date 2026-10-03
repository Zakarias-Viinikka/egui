use super::super::{Line, Outcome, Session};
use crate::general_util::path_parsing::resolve;

pub fn run(args: &[&str], session: &Session) -> Outcome {
    let target = match args.first() {
        None => session.cwd.clone(),
        Some(s) => resolve(s, &session.cwd),
    };

    let entries = match std::fs::read_dir(&target) {
        Ok(e) => e,
        Err(e) => {
            return Outcome::Print(vec![Line::error(format!(
                "ls: {}: {}",
                target.display(),
                e
            ))]);
        }
    };

    let mut lines: Vec<Line> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            lines.push(Line::folder(format!("{name}/")));
        } else {
            lines.push(Line::normal(name));
        }
    }
    lines.sort_by(|a, b| a.text.cmp(&b.text));
    Outcome::Print(lines)
}
