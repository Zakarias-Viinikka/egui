use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::play::{apply_step, read_or_none, Chain};

pub fn apply(chain: &Chain, wanted: &[bool]) -> Result<(), String> {
    for snap in &chain.snapshots {
        let current = read_or_none(&snap.path)?;
        if current != snap.content {
            return Err(format!("{} changed since the chain was built", snap.path.display()));
        }
    }

    let mut contents: HashMap<PathBuf, Option<String>> = HashMap::new();
    for snap in &chain.snapshots {
        contents.insert(snap.path.clone(), snap.content.clone());
    }
    for (i, applied) in chain.steps.iter().enumerate() {
        if !wanted.get(i).copied().unwrap_or(true) {
            continue;
        }
        let current = contents.get(&applied.path).cloned().flatten();
        let after = apply_step(&applied.step, current.as_deref(), &applied.path)?;
        contents.insert(applied.path.clone(), Some(after));
    }

    let mut writes: Vec<(PathBuf, String)> = Vec::new();
    for snap in &chain.snapshots {
        let final_content = contents.get(&snap.path).cloned().flatten();
        if final_content == snap.content {
            continue;
        }
        if let Some(text) = final_content {
            writes.push((snap.path.clone(), text));
        }
    }

    let mut temps: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (path, text) in &writes {
        let tmp = tmp_path_for(path);
        if let Err(e) = std::fs::write(&tmp, text) {
            for (t, _) in &temps {
                let _ = std::fs::remove_file(t);
            }
            return Err(format!("write {}: {}", tmp.display(), e));
        }
        if let Ok(meta) = std::fs::metadata(path) {
            let _ = std::fs::set_permissions(&tmp, meta.permissions());
        }
        temps.push((tmp, path.clone()));
    }

    let mut swapped: Vec<(PathBuf, Option<String>)> = Vec::new();
    for (tmp, target) in &temps {
        let original = chain
            .snapshots
            .iter()
            .find(|s| &s.path == target)
            .and_then(|s| s.content.clone());
        if let Err(e) = std::fs::rename(tmp, target) {
            for (p, orig) in swapped.iter().rev() {
                let _ = restore(p, orig.as_deref());
            }
            for (t, _) in &temps {
                let _ = std::fs::remove_file(t);
            }
            return Err(format!("rename to {}: {}", target.display(), e));
        }
        swapped.push((target.clone(), original));
    }

    Ok(())
}

fn tmp_path_for(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".zap.tmp");
    path.with_file_name(name)
}

fn restore(path: &Path, original: Option<&str>) -> Result<(), String> {
    match original {
        Some(text) => std::fs::write(path, text).map_err(|e| format!("restore {}: {}", path.display(), e)),
        None => {
            let _ = std::fs::remove_file(path);
            Ok(())
        }
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
    fn applies_all_wanted() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![step(Verb::Replace, &file, "hello", "world")], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "world\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skips_denied() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![step(Verb::Replace, &file, "hello", "world")], &dir).expect("play");
        apply(&chain, &[false]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "hello\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_if_file_changed() {
        let dir = tmp();
        let file = dir.join("a.txt");
        fs::write(&file, "hello\n").expect("write");
        let chain = play_forward(vec![step(Verb::Replace, &file, "hello", "world")], &dir).expect("play");
        fs::write(&file, "different\n").expect("write");
        assert!(apply(&chain, &[true]).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_writes_new_file() {
        let dir = tmp();
        let file = dir.join("b.txt");
        let mut s = step(Verb::Create, &file, "", "hi\n");
        s.new_text = "hi\n".to_string();
        let chain = play_forward(vec![s], &dir).expect("play");
        apply(&chain, &[true]).expect("apply");
        assert_eq!(fs::read_to_string(&file).expect("read"), "hi\n");
        let _ = fs::remove_dir_all(&dir);
    }
}
