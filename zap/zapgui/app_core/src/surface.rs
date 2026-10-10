//! Collect the execution surface of a Rust project: everything that runs,
//! or changes what runs, during a build before the user's own program
//! starts. Ordinary source files and target/ are not in it.
//!
//! Nothing here runs cargo, rustc or rustup. Manifests are read with a TOML
//! parser; everything else is a filesystem walk. When unsure, include more.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 12;
const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceFile {
    pub path: PathBuf,
    pub text: String,
}

/// The topmost folder at or above `start` whose Cargo.toml has a
/// [workspace] section, otherwise the nearest Cargo.toml folder.
/// Returns None if no Cargo.toml is found on the way up.
pub fn project_root(start: &Path) -> Option<PathBuf> {
    let start = if start.is_file() { start.parent()? } else { start };
    let mut nearest: Option<PathBuf> = None;
    let mut workspace: Option<PathBuf> = None;
    for ancestor in start.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        if manifest.is_file() {
            if nearest.is_none() {
                nearest = Some(ancestor.to_path_buf());
            }
            if is_workspace(&manifest) {
                workspace = Some(ancestor.to_path_buf());
            }
        }
    }
    workspace.or(nearest)
}

fn is_workspace(manifest: &Path) -> bool {
    read_toml(manifest)
        .and_then(|v| v.get("workspace").cloned())
        .is_some()
}

pub fn collect(start: &Path) -> Result<Vec<SurfaceFile>, String> {
    let Some(root) = project_root(start) else {
        return Err(format!("no Cargo.toml at or above {}", start.display()));
    };
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut files: Vec<SurfaceFile> = Vec::new();
    let mut budget = MAX_TOTAL_BYTES;

    collect_crate(&root, &root, &mut seen, &mut files, &mut budget, 0)?;

    let lock = root.join("Cargo.lock");
    push_file(&lock, &mut seen, &mut files, &mut budget);

    for name in ["rust-toolchain", "rust-toolchain.toml"] {
        push_file(&root.join(name), &mut seen, &mut files, &mut budget);
    }

    let mut dir: Option<&Path> = Some(root.as_path());
    while let Some(d) = dir {
        for name in ["config", "config.toml"] {
            push_file(&d.join(".cargo").join(name), &mut seen, &mut files, &mut budget);
        }
        dir = d.parent();
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn collect_crate(
    crate_dir: &Path,
    workspace_root: &Path,
    seen: &mut BTreeSet<PathBuf>,
    files: &mut Vec<SurfaceFile>,
    budget: &mut usize,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    let manifest = crate_dir.join("Cargo.toml");
    if !manifest.is_file() {
        return Ok(());
    }
    push_file(&manifest, seen, files, budget);

    let Some(value) = read_toml(&manifest) else {
        return Ok(());
    };

    if let Some(ws) = value.get("workspace") {
        if let Some(members) = ws.get("members").and_then(|m| m.as_array()) {
            for m in members {
                let Some(pattern) = m.as_str() else { continue };
                for member_dir in expand_member(crate_dir, pattern) {
                    collect_crate(&member_dir, workspace_root, seen, files, budget, depth + 1)?;
                }
            }
        }
        if let Some(deps) = ws.get("dependencies").and_then(|d| d.as_table()) {
            for (_, spec) in deps.iter() {
                if let Some(p) = spec.get("path").and_then(|p| p.as_str()) {
                    let dep_dir = crate_dir.join(p);
                    collect_crate(&dep_dir, workspace_root, seen, files, budget, depth + 1)?;
                }
            }
        }
    }

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = value.get(section).and_then(|d| d.as_table()) {
            for (_, spec) in deps.iter() {
                if let Some(p) = spec.get("path").and_then(|p| p.as_str()) {
                    let dep_dir = crate_dir.join(p);
                    collect_crate(&dep_dir, workspace_root, seen, files, budget, depth + 1)?;
                }
            }
        }
    }

    let build_rs = crate_dir.join("build.rs");
    if build_rs.is_file() {
        push_file(&build_rs, seen, files, budget);
    }

    if is_proc_macro(&value) {
        walk_dir(&crate_dir.join("src"), seen, files, budget, depth)?;
    }

    Ok(())
}

fn is_proc_macro(value: &toml::Value) -> bool {
    value
        .get("lib")
        .and_then(|l| l.get("proc-macro"))
        .and_then(|p| p.as_bool())
        .unwrap_or(false)
}

fn expand_member(base: &Path, pattern: &str) -> Vec<PathBuf> {
    if !pattern.contains('*') {
        return vec![base.join(pattern)];
    }
    let full = base.join(pattern).to_string_lossy().into_owned();
    let Ok(paths) = glob::glob(&full) else {
        return Vec::new();
    };
    paths.flatten().filter(|p| p.is_dir()).collect()
}

fn walk_dir(
    dir: &Path,
    seen: &mut BTreeSet<PathBuf>,
    files: &mut Vec<SurfaceFile>,
    budget: &mut usize,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH || !dir.is_dir() {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ft.is_dir() {
            walk_dir(&path, seen, files, budget, depth + 1)?;
        } else if ft.is_file() {
            push_file(&path, seen, files, budget);
        }
    }
    Ok(())
}

fn push_file(
    path: &Path,
    seen: &mut BTreeSet<PathBuf>,
    files: &mut Vec<SurfaceFile>,
    budget: &mut usize,
) {
    if !path.is_file() {
        return;
    }
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(canonical.clone()) {
        return;
    }
    let Ok(meta) = std::fs::metadata(&canonical) else { return };
    let size = meta.len() as usize;
    if size > MAX_FILE_BYTES || size > *budget {
        return;
    }
    let Ok(bytes) = std::fs::read(&canonical) else { return };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    *budget -= size;
    files.push(SurfaceFile { path: canonical, text });
}

fn read_toml(path: &Path) -> Option<toml::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("zap_surface_{name}_{}_{}", std::process::id(), nanos));
        fs::create_dir_all(&p).expect("mkdir");
        p
    }

    #[test]
    fn finds_nearest_manifest_without_workspace() {
        let dir = tmpdir("nearest");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let sub = dir.join("src");
        fs::create_dir_all(&sub).unwrap();
        assert_eq!(project_root(&sub).unwrap(), dir);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefers_workspace_above() {
        let dir = tmpdir("ws");
        fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n",
        )
        .unwrap();
        let a = dir.join("a");
        fs::create_dir_all(a.join("src")).unwrap();
        fs::write(
            a.join("Cargo.toml"),
            "[package]\nname = \"a\"\n",
        )
        .unwrap();
        assert_eq!(project_root(&a.join("src")).unwrap(), dir);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn collects_manifest_lock_and_toolchain() {
        let dir = tmpdir("collect");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::write(dir.join("Cargo.lock"), "version = 3\n").unwrap();
        fs::write(dir.join("rust-toolchain.toml"), "[toolchain]\n").unwrap();
        let files = collect(&dir).expect("collect");
        let names: Vec<String> = files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"Cargo.toml".to_string()));
        assert!(names.contains(&"Cargo.lock".to_string()));
        assert!(names.contains(&"rust-toolchain.toml".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn includes_proc_macro_src() {
        let dir = tmpdir("pm");
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"p\"\n\n[lib]\nproc-macro = true\n",
        )
        .unwrap();
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("lib.rs"), "// proc macro\n").unwrap();
        let files = collect(&dir).expect("collect");
        assert!(files.iter().any(|f| f.path.ends_with("src/lib.rs")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn includes_path_dependency() {
        let dir = tmpdir("dep");
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"a\"\n\n[dependencies]\nb = { path = \"b\" }\n",
        )
        .unwrap();
        let b = dir.join("b");
        fs::create_dir_all(&b).unwrap();
        fs::write(b.join("Cargo.toml"), "[package]\nname = \"b\"\n").unwrap();
        let files = collect(&dir).expect("collect");
        assert!(files.iter().any(|f| f.path.ends_with("b/Cargo.toml")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skips_target_dir() {
        let dir = tmpdir("skiptarget");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let t = dir.join("target");
        fs::create_dir_all(&t).unwrap();
        fs::write(t.join("junk"), "x").unwrap();
        let files = collect(&dir).expect("collect");
        assert!(!files.iter().any(|f| f.path.ends_with("junk")));
        let _ = fs::remove_dir_all(&dir);
    }
}
