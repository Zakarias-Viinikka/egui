use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::text_diff::{pair_lines, process, DiffRowProcessedForUi};

use super::step::{Step, Verb};

pub struct AppliedStep {
    pub step: Step,
    pub path: PathBuf,
    pub before: Option<String>,
    pub after: String,
    pub rows: Vec<DiffRowProcessedForUi>,
}

pub struct Snapshot {
    pub path: PathBuf,
    pub content: Option<String>,
}

pub struct Chain {
    pub steps: Vec<AppliedStep>,
    pub snapshots: Vec<Snapshot>,
}

pub fn play_forward(steps: Vec<Step>, cwd: &Path) -> Result<Chain, String> {
    let mut files: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut snapshots: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut order: Vec<PathBuf> = Vec::new();
    let mut applied: Vec<AppliedStep> = Vec::new();

    for step in steps {
        let path = resolve(&step.path, cwd);
        if !files.contains_key(&path) {
            let content = read_or_none(&path)?;
            files.insert(path.clone(), content.clone());
            snapshots.insert(path.clone(), content);
            order.push(path.clone());
        }
        let current = files.get(&path).cloned().flatten();
        let after = apply_step(&step, current.as_deref(), &path)?;
        let before = current;
        let rows = match &before {
            Some(b) => process(&pair_lines(b, &after)),
            None => process(&pair_lines("", &after)),
        };
        files.insert(path.clone(), Some(after.clone()));
        applied.push(AppliedStep { step, path, before, after, rows });
    }

    let snapshots = order
        .into_iter()
        .map(|p| Snapshot { content: snapshots.get(&p).cloned().flatten(), path: p })
        .collect();

    Ok(Chain { steps: applied, snapshots })
}

pub fn apply_step(step: &Step, current: Option<&str>, path: &Path) -> Result<String, String> {
    match step.verb {
        Verb::Create => {
            if current.is_some() {
                return Err(format!("create: {} already exists", path.display()));
            }
            Ok(step.new_text.clone())
        }
        Verb::Overwrite => {
            let content = current.ok_or_else(|| format!("overwrite: {} does not exist", path.display()))?;
            if content == step.new_text {
                return Err(format!("overwrite: {} would not change", path.display()));
            }
            Ok(step.new_text.clone())
        }
        Verb::Replace => {
            let content = current.ok_or_else(|| format!("replace: {} does not exist", path.display()))?;
            if step.old_text.is_empty() {
                return Err("replace: old text is empty".to_string());
            }
            let expected = step.count.unwrap_or(1);
            let found = content.matches(step.old_text.as_str()).count();
            if found != expected {
                return Err(format!(
                    "replace: expected {} occurrence(s) of the old text in {}, found {}",
                    expected,
                    path.display(),
                    found
                ));
            }
            let result = content.replace(step.old_text.as_str(), step.new_text.as_str());
            if result == content {
                return Err(format!("replace: {} would not change", path.display()));
            }
            Ok(result)
        }
    }
}

pub fn read_or_none(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("read {}: {}", path.display(), e)),
    }
}

pub fn resolve(path: &str, cwd: &Path) -> PathBuf {
    if path == "~" {
        return home_dir();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    let p = PathBuf::from(path);
    if p.is_absolute() { p } else { cwd.join(p) }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_play_test_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    fn step(verb: Verb, path: &Path, old: &str, new: &str) -> Step {
        Step {
            verb,
            path: path.display().to_string(),
            old_text: old.to_string(),
            new_text: new.to_string(),
            count: None,
        }
    }

    #[test]
    fn create_then_replace_sees_earlier_step() {
        let dir = tmp();
        let file = dir.join("a.txt");
        let steps = vec![
            step(Verb::Create, &file, "", "hello\n"),
            step(Verb::Replace, &file, "hello", "world"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 2);
        assert_eq!(chain.steps[1].after, "world\n");
        assert_eq!(chain.snapshots.len(), 1);
        assert_eq!(chain.snapshots[0].content, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_fails_when_old_text_missing() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![step(Verb::Replace, &file, "nope", "world")];
        assert!(play_forward(steps, &dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_honours_count() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "x x x\n").expect("write");
        let mut s = step(Verb::Replace, &file, "x", "y");
        s.count = Some(2);
        let steps = vec![s];
        let err = play_forward(steps, &dir).expect_err("should fail");
        assert!(err.contains("expected 2"));
        let _ = fs::remove_dir_all(&dir);
    }
}
