use std::path::{Path, PathBuf};

use crate::last_change::{self, Before, Kind};

use super::play::{read_or_none, AppliedStep, Chain};
use super::step::Verb;

enum Undo {
    RestoreText { path: PathBuf, content: String },
    RemoveFile { path: PathBuf },
    RemoveDir { path: PathBuf },
    CreateDir { path: PathBuf },
    RenameBack { from: PathBuf, to: PathBuf },
    RestoreMode { path: PathBuf, mode: u32 },
}

fn find_step_index(chain: &Chain, path: &Path) -> Option<usize> {
    chain
        .steps
        .iter()
        .position(|s| {
            s.path == path
                || s.dest.as_deref() == Some(path)
                || s.files.iter().any(|f| f.path == path)
        })
}

pub fn apply(chain: &Chain, wanted: &[bool]) -> Result<(), String> {
    apply_detailed(chain, wanted).map_err(|(_, s)| s)
}

/// Like `apply`, but on failure returns the failing step's index
/// (0-based) along with the full reason. Used by the review window so a
/// failure on a hidden step can reveal it.
pub fn apply_detailed(chain: &Chain, wanted: &[bool]) -> Result<(), (usize, String)> {
    let total = chain.steps.len();

    for snap in &chain.snapshots {
        let current = read_or_none(&snap.path)
            .map_err(|e| (find_step_index(chain, &snap.path).unwrap_or(0), e))?;
        if current != snap.content {
            let step_idx = find_step_index(chain, &snap.path).unwrap_or(0);
            let verb = chain
                .steps
                .get(step_idx)
                .map(|s| verb_name(s.step.verb))
                .unwrap_or("step");
            return Err((step_idx, format!(
                "step {} of {} ({} {}) failed: {} changed since the chain was built",
                step_idx + 1,
                total,
                verb,
                snap.path.display(),
                snap.path.display()
            )));
        }
    }

    let mut undos: Vec<Undo> = Vec::new();

    for (i, applied) in chain.steps.iter().enumerate() {
        if !wanted.get(i).copied().unwrap_or(true) {
            continue;
        }
        if let Err(reason) = execute_step(applied, &mut undos) {
            undo_all(&mut undos);
            return Err((i, format!(
                "step {} of {} ({} {}) failed: {}",
                i + 1,
                total,
                verb_name(applied.step.verb),
                applied.path.display(),
                reason
            )));
        }
        if !applied.no_op {
            record_last_change(applied);
        }
    }

    Ok(())
}

fn verb_name(v: Verb) -> &'static str {
    match v {
        Verb::Create => "create",
        Verb::Replace => "replace",
        Verb::Overwrite => "overwrite",
        Verb::Append => "append",
        Verb::Remove => "zaprm",
        Verb::Move => "zapmv",
        Verb::Mkdir => "mkdir",
        Verb::RemoveDir => "rmdir",
        Verb::Cargo => "cargo",
        Verb::Run => "run",
        Verb::Chmod => "chmod",
        Verb::Rename => "rename",
        Verb::AstGrep => "astgrep",
    }
}

fn execute_step(applied: &AppliedStep, undos: &mut Vec<Undo>) -> Result<(), String> {
    if applied.no_op {
        return Ok(());
    }
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
        Verb::RemoveDir => {
            if !applied.path.is_dir() {
                return Err(format!(
                    "{} does not exist or is not a folder",
                    applied.path.display()
                ));
            }
            let mut entries = std::fs::read_dir(&applied.path)
                .map_err(|e| format!("read_dir {}: {}", applied.path.display(), e))?;
            if entries.next().is_some() {
                return Err(format!("{} is not empty", applied.path.display()));
            }
            std::fs::remove_dir(&applied.path)
                .map_err(|e| format!("remove_dir {}: {}", applied.path.display(), e))?;
            undos.push(Undo::CreateDir { path: applied.path.clone() });
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
        Verb::Move | Verb::Rename => {
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
        Verb::Chmod => {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(&applied.path)
                .map_err(|e| format!("metadata {}: {}", applied.path.display(), e))?;
            if meta.is_dir() {
                return Err(format!("{}: is a directory", applied.path.display()));
            }
            let old_mode = meta.permissions().mode() & 0o777;
            let new_mode = exec_mode_for(old_mode);
            std::fs::set_permissions(
                &applied.path,
                std::fs::Permissions::from_mode(new_mode),
            )
            .map_err(|e| format!("chmod {}: {}", applied.path.display(), e))?;
            undos.push(Undo::RestoreMode {
                path: applied.path.clone(),
                mode: old_mode,
            });
            Ok(())
        }
        Verb::AstGrep => {
            // Every file must still read exactly as planned, or nothing
            // is written.
            for f in &applied.files {
                match read_or_none(&f.path)? {
                    Some(current) if current == f.before => {}
                    _ => {
                        return Err(format!(
                            "{} changed since the chain was built",
                            f.path.display()
                        ));
                    }
                }
            }
            for f in &applied.files {
                write_text(&f.path, &f.after, &Some(f.before.clone()), undos)?;
            }
            Ok(())
        }
        Verb::Cargo | Verb::Run => Ok(()),
    }
}

/// Give the owner the execute bit. Group and other get it only where
/// they already have the read bit, so a read-only file stays read-only
/// for everyone else.
fn exec_mode_for(mode: u32) -> u32 {
    let mut out = mode | 0o100;
    if mode & 0o040 != 0 {
        out |= 0o010;
    }
    if mode & 0o004 != 0 {
        out |= 0o001;
    }
    out
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
            Undo::CreateDir { path } => {
                let _ = std::fs::create_dir(&path);
            }
            Undo::RenameBack { from, to } => {
                let _ = std::fs::rename(&from, &to);
            }
            Undo::RestoreMode { path, mode } => {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &path,
                    std::fs::Permissions::from_mode(mode),
                );
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
        Verb::Move | Verb::Rename => {
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
        Verb::AstGrep => {
            for f in &applied.files {
                let before_v = last_change::classify_content(&f.before);
                last_change::record(&f.path, before_v, Kind::Edited, Some(&f.after));
            }
        }
        Verb::Mkdir | Verb::RemoveDir | Verb::Cargo | Verb::Run | Verb::Chmod => {}
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

    fn rmdir_step(path: &Path) -> Step {
        Step {
            verb: Verb::RemoveDir,
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

    #[test]
    fn rmdir_removes_empty_folder() {
        let dir = tmp();
        let sub = dir.join("olddir");
        fs::create_dir_all(&sub).expect("mkdir");
        let chain = play_forward(vec![rmdir_step(&sub)], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert!(!sub.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rmdir_refuses_non_empty_at_write_time() {
        let dir = tmp();
        let sub = dir.join("olddir");
        fs::create_dir_all(&sub).expect("mkdir");
        let chain = play_forward(vec![rmdir_step(&sub)], &dir).expect("play");
        fs::write(sub.join("a.txt"), "x\n").expect("write");
        let err = apply(&chain, &[true]).expect_err("should fail");
        assert!(err.contains("not empty"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_op_overwrite_writes_nothing() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let s = text_step(Verb::Overwrite, &file, "", "hello\n");
        let chain = play_forward(vec![s], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "hello\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_op_then_real_edit_writes_only_the_real_edit() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let steps = vec![
            text_step(Verb::Overwrite, &file, "", "hello\n"),
            text_step(Verb::Replace, &file, "hello", "world"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        apply(&chain, &[true, true]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "world\n");
        let _ = fs::remove_dir_all(&dir);
    }

    fn chmod_step(path: &Path) -> Step {
        Step {
            verb: Verb::Chmod,
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
    fn exec_mode_maps_the_documented_examples() {
        assert_eq!(exec_mode_for(0o644), 0o755);
        assert_eq!(exec_mode_for(0o640), 0o750);
        assert_eq!(exec_mode_for(0o600), 0o700);
        assert_eq!(exec_mode_for(0o400), 0o500);
        assert_eq!(exec_mode_for(0o000), 0o100);
    }

    #[test]
    fn chmod_sets_executable_bit_on_a_real_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp();
        let file = dir.join("a.sh");
        fs::write(&file, "#!/bin/sh\n").expect("write");
        fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        let chain = play_forward(vec![chmod_step(&file)], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        let meta = fs::metadata(&file).expect("meta");
        assert_eq!(meta.permissions().mode() & 0o777, 0o755);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn chmod_undoes_when_a_later_step_fails() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp();
        let file = dir.join("a.sh");
        let sub = dir.join("sub");
        fs::write(&file, "#!/bin/sh\n").expect("write");
        fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        fs::create_dir_all(&sub).expect("mkdir");
        // rmdir a folder that gains a file between play and apply.
        let chain = play_forward(
            vec![chmod_step(&file), rmdir_step(&sub)],
            &dir,
        )
        .expect("play");
        fs::write(sub.join("x.txt"), "x\n").expect("write");
        let err = apply(&chain, &[true, true]).expect_err("should fail");
        assert!(err.contains("not empty"), "got: {err}");
        let meta = fs::metadata(&file).expect("meta");
        assert_eq!(meta.permissions().mode() & 0o777, 0o644);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_detailed_names_the_stale_file_step() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hello\n").expect("write");
        fs::write(&b, "world\n").expect("write");
        let steps = vec![
            text_step(Verb::Replace, &a, "hello", "HELLO"),
            text_step(Verb::Replace, &b, "world", "WORLD"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        fs::write(&b, "stale\n").expect("write");
        let (idx, msg) = apply_detailed(&chain, &[true, true]).expect_err("fail");
        assert_eq!(idx, 1);
        assert!(msg.contains("step 2 of 2"), "got: {msg}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_detailed_names_the_move_step_for_a_stale_dest() {
        let dir = tmp();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hi\n").expect("write");
        let chain = play_forward(vec![mv_step(&a, &b)], &dir).expect("play");
        fs::write(&b, "spawned\n").expect("write");
        let (idx, msg) = apply_detailed(&chain, &[true]).expect_err("fail");
        assert_eq!(idx, 0);
        assert!(msg.contains("step 1 of 1"), "got: {msg}");
        let _ = fs::remove_dir_all(&dir);
    }

    fn astgrep_step(folder: &Path, old: &str, new: &str) -> Step {
        Step {
            verb: Verb::AstGrep,
            path: folder.display().to_string(),
            dest: None,
            old_text: old.to_string(),
            new_text: new.to_string(),
            count: None,
            no_newline: false,
            args: Vec::new(),
            cwd: None,
        }
    }

    /// Two files under a folder and a fake ast-grep that reports `foo` in
    /// both. Returns (folder, a, b).
    fn astgrep_two_files(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        let proj = dir.join("proj");
        fs::create_dir_all(&proj).expect("mkdir");
        let a = proj.join("a.rs");
        let b = proj.join("b.rs");
        fs::write(&a, "fn foo() {}\n").expect("write");
        fs::write(&b, "use x::foo;\n").expect("write");
        let lines = format!(
            "{}{}",
            crate::chain::astgrep::testing::match_line(&a, 3, 6),
            crate::chain::astgrep::testing::match_line(&b, 7, 10)
        );
        crate::chain::astgrep::testing::healthy(dir, &lines);
        (proj, a, b)
    }

    #[test]
    fn astgrep_writes_every_changed_file() {
        let dir = tmp();
        let (proj, a, b) = astgrep_two_files(&dir);
        let chain = play_forward(vec![astgrep_step(&proj, "foo", "bar")], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert_eq!(fs::read_to_string(&a).expect("read"), "fn bar() {}\n");
        assert_eq!(fs::read_to_string(&b).expect("read"), "use x::bar;\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn astgrep_denied_writes_nothing() {
        let dir = tmp();
        let (proj, a, b) = astgrep_two_files(&dir);
        let chain = play_forward(vec![astgrep_step(&proj, "foo", "bar")], &dir).expect("play");
        apply(&chain, &[false]).expect("apply");
        assert_eq!(fs::read_to_string(&a).expect("read"), "fn foo() {}\n");
        assert_eq!(fs::read_to_string(&b).expect("read"), "use x::foo;\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stale_astgrep_file_names_the_astgrep_step_and_writes_nothing() {
        let dir = tmp();
        let (proj, a, b) = astgrep_two_files(&dir);
        let note = dir.join("note.txt");
        let steps = vec![
            text_step(Verb::Create, &note, "", "hi\n"),
            astgrep_step(&proj, "foo", "bar"),
        ];
        let chain = play_forward(steps, &dir).expect("play");
        fs::write(&b, "use x::foo; // edited\n").expect("write");
        let (idx, msg) = apply_detailed(&chain, &[true, true]).expect_err("stale");
        assert_eq!(idx, 1);
        assert!(msg.contains("step 2 of 2 (astgrep"), "got: {msg}");
        assert!(msg.contains("changed since"), "got: {msg}");
        assert_eq!(fs::read_to_string(&a).expect("read"), "fn foo() {}\n");
        assert!(!note.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_still_returns_the_old_message() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(
            vec![text_step(Verb::Replace, &file, "hello", "world")],
            &dir,
        )
        .expect("play");
        fs::write(&file, "different\n").expect("write");
        let err = apply(&chain, &[true]).expect_err("fail");
        assert!(err.contains("step 1 of 1"));
        assert!(err.contains("changed since"));
        let _ = fs::remove_dir_all(&dir);
    }
}
