use db_wrapper::mascot::LiveForever;
use eframe::egui;
use egui_file_dialog::{DialogMode, FileDialog};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use zutil_db::helpers::starred_folders::{star, unstar};

const JUNK_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "__pycache__",
    "venv",
    ".venv",
    ".next",
    ".cache",
    ".parcel-cache",
    ".nuxt",
    ".turbo",
    "coverage",
];

pub fn is_junk(name: &str) -> bool {
    JUNK_NAMES.iter().any(|j| *j == name)
}

#[derive(Clone, Copy, PartialEq)]
pub enum SideMode {
    None,
    S,
    Cdf,
}

pub struct SidePanelState {
    pub mode: SideMode,
    pub filter: String,
    pub junk_filter: bool,
    pub pending_jump: Option<PathBuf>,
    pub starred: Vec<PathBuf>,
}

impl SidePanelState {
    pub fn new(starred: Vec<PathBuf>) -> Self {
        Self {
            mode: SideMode::None,
            filter: String::new(),
            junk_filter: true,
            pending_jump: None,
            starred,
        }
    }

    fn is_starred(&self, path: &Path) -> bool {
        self.starred.iter().any(|p| p == path)
    }

    fn toggle_star(&mut self, path: PathBuf, db: &LiveForever) {
        if let Some(pos) = self.starred.iter().position(|p| p == &path) {
            self.starred.remove(pos);
            if let Err(e) = unstar(db, &path) {
                eprintln!("side_panel::unstar failed: {e}");
            }
        } else {
            if let Err(e) = star(db, &path) {
                eprintln!("side_panel::star failed: {e}");
                return;
            }
            self.starred.push(path);
        }
    }
}

pub fn render_side_panel(ui: &mut egui::Ui, fd: &mut FileDialog, state: &mut SidePanelState, db: &LiveForever) {
    ui.style_mut().spacing.item_spacing = egui::vec2(6.0, 6.0);
    ui.style_mut().spacing.button_padding = egui::vec2(8.0, 4.0);

    if !state.starred.is_empty() {
        ui.label(egui::RichText::new("★ Starred").strong());
        ui.add_space(2.0);

        let snapshot = state.starred.clone();
        let mut to_unstar: Option<PathBuf> = None;
        let mut to_jump: Option<PathBuf> = None;

        for path in &snapshot {
            ui.horizontal(|ui| {
                if ui.small_button("★").on_hover_text("unstar").clicked() {
                    to_unstar = Some(path.clone());
                }
                let label = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                if ui
                    .button(label)
                    .on_hover_text(path.display().to_string())
                    .clicked()
                {
                    to_jump = Some(path.clone());
                }
            });
        }

        if let Some(p) = to_unstar {
            state.toggle_star(p, db);
        }
        if let Some(p) = to_jump {
            state.pending_jump = Some(p);
        }

        ui.add_space(4.0);
        ui.separator();
    }

    egui::CollapsingHeader::new("Disks")
        .default_open(false)
        .show(ui, |ui| {
            let disks = list_disks();
            if disks.is_empty() {
                ui.label("(no mounted disks)");
            } else {
                for (label, mount) in &disks {
                    if ui
                        .button(label)
                        .on_hover_text(mount.display().to_string())
                        .clicked()
                    {
                        state.pending_jump = Some(mount.clone());
                    }
                }
            }
        });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.mode, SideMode::S, "s")
            .on_hover_text("~/ProgStuff");
        ui.selectable_value(&mut state.mode, SideMode::Cdf, "cdf")
            .on_hover_text("subfolders of current folder");
    });

    if state.mode == SideMode::None {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("click s or cdf to browse").weak());
        return;
    }

    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(&mut state.filter)
            .hint_text("filter...")
            .desired_width(f32::INFINITY),
    );
    ui.checkbox(&mut state.junk_filter, "hide junk folders");
    ui.add_space(2.0);
    ui.separator();
    ui.add_space(2.0);

    let folders = match state.mode {
        SideMode::S => walk_s(state.junk_filter),
        SideMode::Cdf => fd
            .storage_mut()
            .last_visited_dir
            .clone()
            .as_deref()
            .map(|d| subdirs_of(d, state.junk_filter))
            .unwrap_or_default(),
        SideMode::None => Vec::new(),
    };

    let filter_lower = state.filter.to_lowercase();
    let filtered: Vec<PathBuf> = folders
        .into_iter()
        .filter(|p| {
            if filter_lower.is_empty() {
                return true;
            }
            p.to_string_lossy().to_lowercase().contains(&filter_lower)
        })
        .collect();

    if filtered.is_empty() {
        ui.label(egui::RichText::new("(empty)").weak());
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for path in &filtered {
                ui.horizontal(|ui| {
                    let starred = state.is_starred(path.as_path());
                    let star_char = if starred { "★" } else { "☆" };
                    if ui
                        .small_button(star_char)
                        .on_hover_text(if starred { "unstar" } else { "star" })
                        .clicked()
                    {
                        state.toggle_star(path.clone(), db);
                    }
                    let label = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .to_string();
                    if ui
                        .button(label)
                        .on_hover_text(path.display().to_string())
                        .clicked()
                    {
                        state.pending_jump = Some(path.clone());
                    }
                });
            }
        });
}

pub fn jump_picker_to(fd: &mut FileDialog, path: PathBuf) {
    let was_save = matches!(fd.mode(), DialogMode::SaveFile);
    fd.config_mut().initial_directory = path.clone();
    fd.storage_mut().last_visited_dir = Some(path.clone());
    fd.storage_mut().last_picked_dir = Some(path);
    if was_save {
        fd.save_file();
    } else {
        fd.pick_file();
    }
}

fn subdirs_of(dir: &Path, junk_filter: bool) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| {
            if !junk_filter {
                return true;
            }
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| !is_junk(n))
                .unwrap_or(true)
        })
        .collect()
}

fn walk_s(junk_filter: bool) -> Vec<PathBuf> {
    let home = match std::env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => return Vec::new(),
    };
    let root = home.join("ProgStuff");
    if !root.is_dir() {
        return Vec::new();
    }
    let mut result = vec![root.clone()];
    walk_subdirs(&root, junk_filter, 4, &mut result);
    result
}

fn walk_subdirs(dir: &Path, junk_filter: bool, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if junk_filter {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if is_junk(name) {
                    continue;
                }
            }
        }
        out.push(path.clone());
        walk_subdirs(&path, junk_filter, depth - 1, out);
    }
}

fn list_disks() -> Vec<(String, PathBuf)> {
    if let Some(result) = list_disks_via_lsblk() {
        if !result.is_empty() {
            return result;
        }
    }
    list_disks_from_mounts()
}

fn list_disks_via_lsblk() -> Option<Vec<(String, PathBuf)>> {
    let output = std::process::Command::new("lsblk")
        .args(["-P", "-n", "-o", "NAME,LABEL,MOUNTPOINT,TYPE"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut result: Vec<(String, PathBuf)> = Vec::new();
    for line in text.lines() {
        let fields = parse_kv_line(line);
        let mount = fields.get("MOUNTPOINT").cloned().unwrap_or_default();
        if mount.is_empty() {
            continue;
        }
        let kind = fields.get("TYPE").cloned().unwrap_or_default();
        if kind == "loop" || kind == "rom" {
            continue;
        }
        let label = fields.get("LABEL").cloned().unwrap_or_default();
        let name = fields.get("NAME").cloned().unwrap_or_default();
        let display = if !label.is_empty() {
            format!("{} (/dev/{})", label, name)
        } else {
            format!("{} (/dev/{})", mount, name)
        };
        let mount_path = PathBuf::from(&mount);
        if result.iter().any(|(_, m)| m == &mount_path) {
            continue;
        }
        result.push((display, mount_path));
    }
    Some(result)
}

fn parse_kv_line(line: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for token in line.split_whitespace() {
        if let Some((k, v)) = token.split_once('=') {
            let v = v.trim_matches('"');
            out.insert(k.to_string(), v.to_string());
        }
    }
    out
}

fn list_disks_from_mounts() -> Vec<(String, PathBuf)> {
    let Ok(content) = fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let device = parts[0];
        let mount = parts[1].replace("\\040", " ");
        let fstype = parts[2];
        if !device.starts_with("/dev/") {
            continue;
        }
        let allowed = matches!(
            fstype,
            "ext4" | "ext3" | "ext2" | "xfs" | "btrfs" | "vfat" | "exfat"
                | "ntfs" | "ntfs3" | "iso9660" | "udf"
        ) || fstype.starts_with("fuse.");
        if !allowed {
            continue;
        }
        let mount_path = PathBuf::from(&mount);
        if out.iter().any(|(_, m)| m == &mount_path) {
            continue;
        }
        out.push((format!("{} → {}", device, mount), mount_path));
    }
    out
}
