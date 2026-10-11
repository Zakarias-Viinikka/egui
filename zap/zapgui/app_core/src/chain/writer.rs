use std::path::{Path, PathBuf};

use crate::last_change::{self, Before, Kind};

use super::play::{read_or_none, AppliedStep, Chain};
use super::step::Verb;

enum Undo {
    RestoreText { path: PathBuf, content: String },
    RemoveFile { path: PathBuf },
    RemoveDir { path: PathBuf },
    RenameBack { from: PathBuf, to: PathBuf },
}

pub fn apply(chain: &Chain, wanted: &[bool]) -> Result<(), String> {
    let total = chain.steps.len();

    // Re-verify snapshots. If anything changed between review and confirm,
    // refuse rather than clobber.
    for snap in &chain.snapshots {
        let current = read_or_none(&snap.path)?;
        if current != snap.content {
            let step_no = chain
                .steps
                .iter()
                .position(|s| s.path == snap.path)
                .map(|i| i + 1)
                .unwrap_or(0);
            let verb = chain
                .steps
                .iter()
                .find(|s| s.path == snap.path)
                .map(|s| verb_name(s.step.verb))
                .unwrap_or("step");
            return Err(format!(
                "step {} of {} ({} {}) failed: {} changed since the chain was built",
                step_no,
                total,
                verb,
                snap.path.display(),
                snap.path.display()
            ));
        }
    }

    let mut undos: Vec<Undo> = Vec::new();

    for (i, applied) in chain.steps.iter().enumerate() {
        if !wanted.get(i).copied().unwrap_or(true) {
            continue;
        }
        let step_no = i + 1;
        if let Err(reason) = execute_step(applied, &mut undos) {
            undo_all(&mut undos);
            return Err(format!(
                "step {} of {} ({} {}) failed: {}",
                step_no,
                total,
                verb_name(applied.step.verb),
                applied.path.display(),
                reason
            ));
        }
        record_last_change(applied);
    }

    Ok(())
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

fn execute_step(applied: &AppliedStep, undos: &mut Vec<Undo>) -> Result<(), String> {
    match applied.step.verb {
        Verb::Mkdir => {
            if applied.path.exists() {
                return Err(format!("{} already exists", applied.path.display()));
            }
            std::fs::create_dir_all(&applied.path)
                .map_err(|e| format!("create_dir_all {}: {}", applied.path.display(), e))?;
            undos.push(Undo::RemoveDir { path: applied.path.clone() });
            Ok(())
        }
        Verb::Remove => {
            let before = applied
                .before
                .clone()
                .ok_or_else(|| format!("no before content for {}", applied.path.display()))?;
            std::fs::remove_file(&applied.path)
                .map_err(|e| format!("remove_file {}: {}", applied.path.display(), e))?;
            undos.push(Undo::RestoreText {
                path: applied.path.clone(),
                content: before,
            });
            Ok(())
        }
        Verb::Move => {
            let dest = applied
                .dest
                .clone()
                .ok_or_else(|| "no resolved destination".to_string())?;
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("create_dir_all {}: {}", parent.display(), e))?;
            }
            match std::fs::rename(&applied.path, &dest) {
                Ok(()) => {}
                Err(_) => {
                    let bytes = std::fs::read(&applied.path)
                        .map_err(|e| format!("read {}: {}", applied.path.display(), e))?;
                    std::fs::write(&dest, bytes)
                        .map_err(|e| format!("write {}: {}", dest.display(), e))?;
                    std::fs::remove_file(&applied.path)
                        .map_err(|e| format!("remove_file {}: {}", applied.path.display(), e))?;
                }
            }
            undos.push(Undo::RenameBack {
                from: dest,
                to: applied.path.clone(),
            });
            Ok(())
        }
        Verb::Create | Verb::Replace | Verb::Overwrite | Verb::Append => {
            write_text(&applied.path, &applied.after, &applied.before, undos)
        }
        Verb::Cargo | Verb::Run => Ok(()),
    }
}

fn write_text(
    path: &Path,
    new_text: &str,
    before: &Option<String>,
    undos: &mut Vec<Undo>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create_dir_all {}: {}", parent.display(), e))?;
            undos.push(Undo::RemoveDir {
                path: parent.to_path_buf(),
            });
        }
    }

    let tmp = tmp_path_for(path);
    std::fs::write(&tmp, new_text)
        .map_err(|e| format!("write {}: {}", tmp.display(), e))?;
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("rename to {}: {}", path.display(), e))?;

    match before {
        Some(old) => undos.push(Undo::RestoreText {
            path: path.to_path_buf(),
            content: old.clone(),
        }),
        None => undos.push(Undo::RemoveFile {
            path: path.to_path_buf(),
        }),
    }
    Ok(())
}

fn undo_all(undos: &mut Vec<Undo>) {
    while let Some(u) = undos.pop() {
        match u {
            Undo::RestoreText { path, content } => {
                let _ = std::fs::write(&path, content);
            }
            Undo::RemoveFile { path } => {
                let _ = std::fs::remove_file(&path);
            }
            Undo::RemoveDir { path } => {
                let _ = std::fs::remove_dir(&path);
            }
            Undo::RenameBack { from, to } => {
                let _ = std::fs::rename(&from, &to);
            }
        }
    }
}

fn tmp_path_for(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".zap.tmp");
    path.with_file_name(name)
}

fn record_last_change(applied: &AppliedStep) {
    match applied.step.verb {
        Verb::Create => {
            last_change::record(
                &applied.path,
                Before::Missing,
                Kind::Created,
                Some(&applied.after),
            );
        }
        Verb::Overwrite | Verb::Replace | Verb::Append => {
            let before_text = applied.before.clone().unwrap_or_default();
            let before_v = last_change::classify_content(&before_text);
            last_change::record(
                &applied.path,
                before_v,
                Kind::Edited,
                Some(&applied.after),
            );
        }
        Verb::Remove => {
            let before_text = applied.before.clone().unwrap_or_default();
            let before_v = last_change::classify_content(&before_text);
            last_change::record(&applied.path, before_v, Kind::Removed, None);
        }
        Verb::Move => {
            let dest = applied.dest.clone().unwrap_or_default();
            let content = applied.before.clone().unwrap_or_default();
            let content_v = last_change::classify_content(&content);
            last_change::record(
                &dest,
                content_v.clone(),
                Kind::RenamedFrom(applied.path.clone()),
                Some(&content),
            );
            last_change::record(
                &applied.path,
                content_v,
                Kind::RenamedTo(dest),
                None,
            );
        }
        Verb::Mkdir | Verb::Cargo | Verb::Run => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::play::play_forward;
    use crate::chain::step::{Step, Verb};
    use std::fs;

    fn tmp() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_writer_test_{}_{}", std::process::id(), nanos));
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
    fn applies_all_wanted() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![text_step(Verb::Replace, &file, "hello", "world")], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "world\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skips_denied() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![text_step(Verb::Replace, &file, "hello", "world")], &dir).expect("play");
        apply(&chain, &[false]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "hello\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_if_file_changed_and_names_the_step() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![text_step(Verb::Replace, &file, "hello", "world")], &dir).expect("play");
        fs::write(&file, "different\n").expect("write");
        let err = apply(&chain, &[true]).expect_err("should fail");
        assert!(err.contains("step 1 of 1"), "got: {err}");
        assert!(err.contains("replace"));
        assert!(err.contains("changed since"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_writes_new_file() {
        let dir = tmp();
        let file = dir.join("b.txt");
        let mut s = text_step(Verb::Create, &file, "", "hi\n");
        s.new_text = "hi\n".to_string();
        let chain = play_forward(vec![s], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "hi\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rm_removes_the_file() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![rm_step(&file)], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert!(!file.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mv_moves_the_file() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hello\n").expect("write");
        let chain = play_forward(vec![mv_step(&a, &b)], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert!(!a.exists());
        assert_eq!(fs::read_to_string(&b).expect("read"), "hello\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mkdir_makes_the_folder() {
        let dir = tmp();
        let sub = dir.join("newdir");
        let chain = play_forward(vec![mkdir_step(&sub)], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert!(sub.is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn applies_full_chain_and_undoes_nothing() {
        let dir = tmp();
        let sub = dir.join("newdir");
        let file = sub.join("a.txt");
        let moved = sub.join("b.txt");
        let steps = vec![
            mkdir_step(&sub),
            text_step(Verb::Create, &file, "", "hi\n"),
            mv_step(&file, &moved),
            rm_step(&moved),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        apply(&chain, &[true; 4]).expect("apply");
        assert!(sub.is_dir());
        assert!(!moved.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}