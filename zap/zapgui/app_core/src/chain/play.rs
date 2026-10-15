use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use crate::text_diff::moves as move_detect;
use crate::text_diff::{pair_lines, pair_lines_full, process, DiffRowProcessedForUi, MoveSpan};

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
    /// Move spans detected across the whole chain. Empty until move
    /// detection lands.
    pub move_spans: Vec<MoveSpan>,
}

pub struct Snapshot {
    pub path: PathBuf,
    pub content: Option<String>,
}

pub struct Chain {
    pub steps: Vec<AppliedStep>,
    pub snapshots: Vec<Snapshot>,
    /// Set only for chains that arrive over a ChainRun frame. A streaming
    /// handler puts a sender here and waits for the outcome. Wrapped in a
    /// Mutex because mpsc::Sender is Send but not Sync, and Chain is
    /// stored inside an Arc.
    pub notify: Option<Mutex<Sender<ChainOutcome>>>,
}

#[derive(Clone, Debug)]
pub enum ChainOutcome {
    /// Some steps applied, some denied. The booleans match the steps:
    /// true means applied.
    Applied(Vec<bool>),
    /// Every step denied. Nothing was written.
    AllDenied,
    /// A newer chain replaced this one before the user acted.
    Replaced,
}

impl Chain {
    /// True if any step needs review even when every file is whitelisted.
    /// A run step joins the rm/mv rule because the run is a side effect
    /// that has to be confirmed.
    pub fn needs_review(&self) -> bool {
        self.steps.iter().any(|s| {
            matches!(
                s.step.verb,
                Verb::Remove | Verb::Move | Verb::Cargo | Verb::Run
            )
        })
    }

    /// Send the outcome to the waiting handler, if one is there.
    pub fn finish(&self, outcome: ChainOutcome) {
        if let Some(m) = &self.notify {
            if let Ok(tx) = m.lock() {
                let _ = tx.send(outcome);
            }
        }
    }
}

pub fn play_forward(steps: Vec<Step>, cwd: &Path) -> Result<Chain, String> {
    let mut files: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut snapshots: HashMap<PathBuf, Option<String>> = HashMap::new();
    let mut order: Vec<PathBuf> = Vec::new();
    let mut dirs: HashSet<PathBuf> = HashSet::new();
    let mut applied: Vec<AppliedStep> = Vec::new();

    let total = steps.len();
    for (idx, step) in steps.into_iter().enumerate() {
        let step_no = idx + 1;
        let verb = verb_name(step.verb);
        let step_path = step.path.clone();

        let result = play_one(
            step,
            cwd,
            &mut files,
            &mut snapshots,
            &mut order,
            &mut dirs,
            &mut applied,
        );
        if let Err(reason) = result {
            return Err(format!(
                "step {} of {} ({} {}) failed: {}",
                step_no, total, verb, step_path, reason
            ));
        }
    }

    // Move detection across all steps at once. Each step's rows plus its
    // display path go in; the returned per-step spans get stored on the
    // AppliedSteps.
    let step_inputs: Vec<(Vec<crate::text_diff::DiffRow>, String)> = applied
        .iter()
        .map(|s| {
            let before = s.before.as_deref().unwrap_or("");
            let rows = pair_lines_full(before, &s.after);
            (rows, s.path.display().to_string())
        })
        .collect();
    let all_spans = move_detect::detect(&step_inputs);
    for (i, spans) in all_spans.into_iter().enumerate() {
        if let Some(slot) = applied.get_mut(i) {
            slot.move_spans = spans;
        }
    }

    let snapshots = order
        .into_iter()
        .map(|p| Snapshot { content: snapshots.get(&p).cloned().flatten(), path: p })
        .collect();

    Ok(Chain { steps: applied, snapshots, notify: None })
}

fn verb_name(v: Verb) -> &'static str {
    match v {
        Verb::Create => "create",
        Verb::Replace => "replace",
        Verb::Overwrite => "overwrite",
        Verb::Append => "append",
        Verb::Remove => "delete",
        Verb::Move => "rename",
        Verb::Mkdir => "mkdir",
        Verb::Cargo => "cargo",
        Verb::Run => "run",
    }
}

#[allow(clippy::too_many_arguments)]
fn play_one(
    step: Step,
    cwd: &Path,
    files: &mut HashMap<PathBuf, Option<String>>,
    snapshots: &mut HashMap<PathBuf, Option<String>>,
    order: &mut Vec<PathBuf>,
    dirs: &mut HashSet<PathBuf>,
    applied: &mut Vec<AppliedStep>,
) -> Result<(), String> {
    let path = resolve(&step.path, cwd);

    match step.verb {
        Verb::Mkdir => {
            if path.exists()
                || files.get(&path).cloned().flatten().is_some()
                || dirs.contains(&path)
            {
                return Err(format!("{} already exists", path.display()));
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
                move_spans: Vec::new(),
            });
        }
        Verb::Remove => {
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_dir() {
                    return Err(format!("{}: is a directory", path.display()));
                }
                if meta.len() > RM_MAX_BYTES {
                    return Err(
                        "can't keep an undo copy of this file; delete it by hand"
                            .to_string(),
                    );
                }
            }
            load_once(&path, files, snapshots, order)?;
            let before = files.get(&path).cloned().flatten();
            let Some(content) = before.clone() else {
                return Err(format!("{} does not exist", path.display()));
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
                move_spans: Vec::new(),
            });
        }
        Verb::Move => {
            load_once(&path, files, snapshots, order)?;
            let dest_str = step
                .dest
                .as_ref()
                .ok_or_else(|| "missing destination".to_string())?;
            let dest = resolve(dest_str, cwd);
            if dest == path {
                return Err(format!("{} is the same as the source", path.display()));
            }
            let source_content = files.get(&path).cloned().flatten();
            let Some(content) = source_content.clone() else {
                return Err(format!("{} does not exist", path.display()));
            };
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_dir() {
                    return Err(format!("{}: is a directory", path.display()));
                }
            }
            if files.get(&dest).cloned().flatten().is_some() || dest.exists() {
                return Err(format!("{} already exists", dest.display()));
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
                move_spans: Vec::new(),
            });
        }
        Verb::Create | Verb::Replace | Verb::Overwrite | Verb::Append => {
            load_once(&path, files, snapshots, order)?;
            let current = files.get(&path).cloned().flatten();
            let after = apply_step(&step, current.as_deref(), &path)?;
            let before = current;
            let rows = match &before {
                Some(b) => process(&pair_lines(b, &after)),
                None => process(&pair_lines("", &after)),
            };
            files.insert(path.clone(), Some(after.clone()));
            applied.push(AppliedStep {
                step,
                path,
                dest: None,
                before,
                after,
                rows,
                move_spans: Vec::new(),
            });
        }
        Verb::Cargo | Verb::Run => {
            let args = step.args.join(" ");
            let folder_raw = step.cwd.clone().unwrap_or_else(|| step.path.clone());
            let folder = resolve(&folder_raw, cwd);
            let cmd_line = match step.verb {
                Verb::Cargo => format!("cargo {args}"),
                Verb::Run => format!("run {args}"),
                _ => unreachable!(),
            };
            let after = format!("{cmd_line}\nin {}", folder.display());
            let rows = process(&pair_lines("", &after));
            applied.push(AppliedStep {
                step,
                path: folder,
                dest: None,
                before: None,
                after,
                rows,
                move_spans: Vec::new(),
            });
        }
    }

    Ok(())
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
                return Err(format!("{} already exists", path.display()));
            }
            Ok(step.new_text.clone())
        }
        Verb::Overwrite => {
            let content =
                current.ok_or_else(|| format!("{} does not exist", path.display()))?;
            if content == step.new_text {
                return Err(format!("{} would not change", path.display()));
            }
            Ok(step.new_text.clone())
        }
        Verb::Replace => {
            let content =
                current.ok_or_else(|| format!("{} does not exist", path.display()))?;
            if step.old_text.is_empty() {
                return Err("old text is empty".to_string());
            }
            let expected = step.count.unwrap_or(1);
            let found = content.matches(step.old_text.as_str()).count();
            if found != expected {
                return Err(format!(
                    "expected {} occurrence(s) of the old text in {}, found {}",
                    expected,
                    path.display(),
                    found
                ));
            }
            let result = content.replace(step.old_text.as_str(), step.new_text.as_str());
            if result == content {
                return Err(format!("{} would not change", path.display()));
            }
            Ok(result)
        }
        Verb::Append => {
            let content =
                current.ok_or_else(|| format!("{} does not exist", path.display()))?;
            if step.new_text.is_empty() {
                return Err(format!("{} would not change", path.display()));
            }
            let mut out = content.to_string();
            if step.no_newline {
                out.push_str(&step.new_text);
            } else {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(&step.new_text);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            if out == content {
                return Err(format!("{} would not change", path.display()));
            }
            Ok(out)
        }
        Verb::Remove | Verb::Move | Verb::Mkdir | Verb::Cargo | Verb::Run => {
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
            no_newline: false,
            args: Vec::new(),
            cwd: None,
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
            no_newline: false,
            args: Vec::new(),
            cwd: None,
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
            no_newline: false,
            args: Vec::new(),
            cwd: None,
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
            no_newline: false,
            args: Vec::new(),
            cwd: None,
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
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.contains("step 1 of 1"));
        assert!(err.contains("replace"));
        assert!(err.contains("expected 1 occurrence"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failure_names_the_step_index() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hello\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &a, "hello", "world"),
            text_step(Verb::Replace, &b, "nope", "world"),
        ];
        let err = match play_forward(steps, &dir) {
            Ok(_) => panic!("should fail"),
            Err(e) => e,
        };
        assert!(err.starts_with("step 2 of 2"), "got: {err}");
        assert!(err.contains("replace"));
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

    #[test]
    fn append_adds_a_line() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "one\n").expect("write");
        let mut s = text_step(Verb::Append, &file, "", "two");
        s.new_text = "two".to_string();
        let chain = play_forward(vec![s], &dir).expect("play");
        assert_eq!(chain.steps[0].after, "one\ntwo\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_inserts_newline_if_missing() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "one").expect("write");
        let mut s = text_step(Verb::Append, &file, "", "two");
        s.new_text = "two".to_string();
        let chain = play_forward(vec![s], &dir).expect("play");
        assert_eq!(chain.steps[0].after, "one\ntwo\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_no_newline_concatenates() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "one").expect("write");
        let mut s = text_step(Verb::Append, &file, "", "two");
        s.new_text = "two".to_string();
        s.no_newline = true;
        let chain = play_forward(vec![s], &dir).expect("play");
        assert_eq!(chain.steps[0].after, "onetwo");
        let _ = fs::remove_dir_all(&dir);
    }
}