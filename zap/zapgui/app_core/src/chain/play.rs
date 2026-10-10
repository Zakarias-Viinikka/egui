
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::text_diff::{pair_lines, process, DiffRowProcessedForUi};

use super::step::{Step, Verb};

/// rm refuses to keep an undo copy of a file larger than this.
const RM_MAX_BYTES: u64 = 1024 * 1024;

pub struct AppliedStep {
    pub step: Step,
    pub path: PathBuf,
    /// Move only: the destination, resolved against the chain's cwd.
    pub dest: Option<PathBuf>,
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
    let mut dirs: HashSet<PathBuf> = HashSet::new();
    let mut applied: Vec<AppliedStep> = Vec::new();

    for step in steps {
        let path = resolve(&step.path, cwd);

        match step.verb {
            Verb::Mkdir => {
                if path.exists() || files.get(&path).cloned().flatten().is_some() || dirs.contains(&path) {
                    return Err(format!("mkdir: {} already exists", path.display()));
                }
                dirs.insert(path.clone());
                let after = format!("(new folder) {}/", path.display());
                let rows = process(&pair_lines("", &after));
                applied.push(AppliedStep {
                    step,
                    path,
                    dest: None,
                    before: None,
                    after,
                    rows,
                });
            }
            Verb::Remove => {
                if let Ok(meta) = std::fs::metadata(&path) {
                    if meta.is_dir() {
                        return Err(format!("rm: {}: is a directory", path.display()));
                    }
                    if meta.len() > RM_MAX_BYTES {
                        return Err(
                            "rm: can't keep an undo copy of this file; delete it by hand"
                                .to_string(),
                        );
                    }
                }
                load_once(&path, &mut files, &mut snapshots, &mut order)?;
                let before = files.get(&path).cloned().flatten();
                let Some(content) = before.clone() else {
                    return Err(format!("rm: {} does not exist", path.display()));
                };
                let rows = process(&pair_lines(&content, ""));
                files.insert(path.clone(), None);
                applied.push(AppliedStep {
                    step,
                    path,
                    dest: None,
                    before: Some(content),
                    after: String::new(),
                    rows,
                });
            }
            Verb::Move => {
                load_once(&path, &mut files, &mut snapshots, &mut order)?;
                let dest_str = step
                    .dest
                    .as_ref()
                    .ok_or_else(|| "mv: missing destination".to_string())?;
                let dest = resolve(dest_str, cwd);
                if dest == path {
                    return Err(format!("mv: {} is the same as the source", path.display()));
                }
                let source_content = files.get(&path).cloned().flatten();
                let Some(content) = source_content.clone() else {
                    return Err(format!("mv: {} does not exist", path.display()));
                };
                if let Ok(meta) = std::fs::metadata(&path) {
                    if meta.is_dir() {
                        return Err(format!("mv: {}: is a directory", path.display()));
                    }
                }
                if files.get(&dest).cloned().flatten().is_some() || dest.exists() {
                    return Err(format!("mv: {} already exists", dest.display()));
                }
                if !files.contains_key(&dest) {
                    files.insert(dest.clone(), None);
                    snapshots.insert(dest.clone(), None);
                    order.push(dest.clone());
                }
                files.insert(dest.clone(), Some(content.clone()));
                let rows = process(&pair_lines(&content, &content));
                files.insert(path.clone(), None);
                applied.push(AppliedStep {
                    step,
                    path,
                    dest: Some(dest.clone()),
                    before: Some(content.clone()),
                    after: content,
                    rows,
                });
            }
            Verb::Create | Verb::Replace | Verb::Overwrite => {
                load_once(&path, &mut files, &mut snapshots, &mut order)?;
                let current = files.get(&path).cloned().flatten();
                let after = apply_step(&step, current.as_deref(), &path)?;
                let before = current;
                let rows = match &before {
                    Some(b) => process(&pair_lines(b, &after)),
                    None => process(&pair_lines("", &after)),
                };
                files.insert(path.clone(), Some(after.clone()));
                applied.push(AppliedStep { step, path, dest: None, before, after, rows });
            }
        }
    }

    let snapshots = order
        .into_iter()
        .map(|p| Snapshot { content: snapshots.get(&p).cloned().flatten(), path: p })
        .collect();

    Ok(Chain { steps: applied, snapshots })
}

fn load_once(
    path: &Path,
    files: &mut HashMap<PathBuf, Option<String>>,
    snapshots: &mut HashMap<PathBuf, Option<String>>,
    order: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if files.contains_key(path) {
        return Ok(());
    }
    let content = read_or_none(path)?;
    files.insert(path.to_path_buf(), content.clone());
    snapshots.insert(path.to_path_buf(), content);
    order.push(path.to_path_buf());
    Ok(())
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
            let content = current
                .ok_or_else(|| format!("overwrite: {} does not exist", path.display()))?;
            if content == step.new_text {
                return Err(format!("overwrite: {} would not change", path.display()));
            }
            Ok(step.new_text.clone())
        }
        Verb::Replace => {
            let content = current
                .ok_or_else(|| format!("replace: {} does not exist", path.display()))?;
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
        Verb::Remove | Verb::Move | Verb::Mkdir => {
            Err("apply_step: this verb is handled in play_forward".to_string())
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

    fn text_step(verb: Verb, path: &Path, old: &str, new: &str) -> Step {
        Step {
            verb,
            path: path.display().to_string(),
            dest: None,
            old_text: old.to_string(),
            new_text: new.to_string(),
            count: None,
        }
    }

    fn mv_step(from: &Path, to: &Path) -> Step {
        Step {
            verb: Verb::Move,
            path: from.display().to_string(),
            dest: Some(to.display().to_string()),
            old_text: String::new(),
            new_text: String::new(),
            count: None,
        }
    }

    fn rm_step(path: &Path) -> Step {
        Step {
            verb: Verb::Remove,
            path: path.display().to_string(),
            dest: None,
            old_text: String::new(),
            new_text: String::new(),
            count: None,
        }
    }

    fn mkdir_step(path: &Path) -> Step {
        Step {
            verb: Verb::Mkdir,
            path: path.display().to_string(),
            dest: None,
            old_text: String::new(),
            new_text: String::new(),
            count: None,
        }
    }

    #[test]
    fn create_then_replace_sees_earlier_step() {
        let dir = tmp();
        let file = dir.join("a.txt");
        let steps = vec![
            text_step(Verb::Create, &file, "", "hello\n"),
            text_step(Verb::Replace, &file, "hello", "world"),
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
        let steps = vec![text_step(Verb::Replace, &file, "nope", "world")];
        assert!(play_forward(steps, &dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_honours_count() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "x x x\n").expect("write");
        let mut s = text_step(Verb::Replace, &file, "x", "y");
        s.count = Some(2);
        let steps = vec![s];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("expected 2"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rm_needs_the_file_to_exist() {
        let dir = tmp();
        let file = dir.join("a.txt");
        let steps = vec![rm_step(&file)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("does not exist"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rm_refuses_directories() {
        let dir = tmp();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).expect("mkdir");
        let steps = vec![rm_step(&sub)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("is a directory"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rm_then_create_sees_empty() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![
            rm_step(&file),
            text_step(Verb::Create, &file, "", "new\n"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps[1].after, "new\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mv_needs_source_and_dest_checks() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hi\n").expect("write");
        let steps = vec![mv_step(&a, &b)];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].after, "hi\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mv_refuses_existing_destination() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hi\n").expect("write");
        fs::write(&b, "other\n").expect("write");
        let steps = vec![mv_step(&a, &b)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("already exists"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mv_onto_itself_is_an_error() {
        let dir = tmp();
        let a = dir.join("a.txt");
        fs::write(&a, "hi\n").expect("write");
        let steps = vec![mv_step(&a, &a)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("same as the source"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mkdir_refuses_existing() {
        let dir = tmp();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).expect("mkdir");
        let steps = vec![mkdir_step(&sub)];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("already exists"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mkdir_then_create_inside() {
        let dir = tmp();
        let sub = dir.join("newdir");
        let file = sub.join("a.txt");
        let steps = vec![
            mkdir_step(&sub),
            text_step(Verb::Create, &file, "", "hi\n"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        assert_eq!(chain.steps.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }
}
