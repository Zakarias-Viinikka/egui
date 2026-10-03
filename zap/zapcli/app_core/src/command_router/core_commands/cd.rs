use super::super::{Line, Outcome, Session};
use crate::general_util::path_parsing::{home_dir, resolve};

pub fn run(args: &[&str], session: &mut Session) -> Outcome {
    let target = match args.first() {
        None => home_dir(),
        Some(s) => resolve(s, &session.cwd),
    };

    match std::fs::canonicalize(&target) {
        Ok(p) if p.is_dir() => {
            session.cwd = p;
            Outcome::Silent
        }
        _ => Outcome::Print(vec![Line::error(format!(
            "cd: {}: no such directory",
            target.display()
        ))]),
    }
}
