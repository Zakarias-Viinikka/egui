use crate::error::{AppError, AppResult};
use crate::kdenlive::info::TimelineClipInfo;
use crate::kdenlive::talker::{build_proxy, connect};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct PulledClip {
    pub original_clips: Vec<TimelineClipInfo>,
    pub start_position: i32,
    pub track_id: i32,
    pub temp_file: PathBuf,
}

impl PulledClip {
    pub fn original_ids(&self) -> Vec<i32> {
        self.original_clips.iter().map(|i| i.id).collect()
    }

    pub fn display_name(&self) -> String {
        if self.original_clips.len() == 1 {
            self.original_clips[0].name.clone()
        } else {
            format!(
                "{} (+{} more)",
                self.original_clips[0].name,
                self.original_clips.len() - 1
            )
        }
    }
}

pub fn pull_from_kdenlive(output_dir: &Path) -> AppResult<PulledClip> {
    eprintln!("[video_lib] connecting to kdenlive");
    let (conn, name) = connect()?;
    eprintln!("[video_lib] connected to {name}");
    let proxy = build_proxy(&conn, &name)?;

    let selection = proxy
        .script_get_selection()
        .map_err(|e| AppError::new(format!("get_selection: {e}"), "kdenlive::pull", file!()))?;

    let ids: Vec<i32> = selection
        .iter()
        .filter_map(|v| v.downcast_ref::<i32>().ok())
        .collect();
    eprintln!("[video_lib] selection: {:?}", ids);

    if ids.is_empty() {
        return Err(AppError::new("nothing selected in Kdenlive", "kdenlive::pull", file!()));
    }

    let fps = proxy
        .script_get_project_fps()
        .map_err(|e| AppError::new(format!("get_fps: {e}"), "kdenlive::pull", file!()))?;
    if fps <= 0.0 {
        return Err(AppError::new(format!("bad fps {fps}"), "kdenlive::pull", file!()));
    }

    let mut infos: Vec<TimelineClipInfo> = Vec::new();
    for id in &ids {
        let map = proxy
            .script_get_timeline_clip_info(*id)
            .map_err(|e| AppError::new(format!("get_clip_info({id}): {e}"), "kdenlive::pull", file!()))?;
        let info = TimelineClipInfo::from_dbus(&map)?;
        eprintln!(
            "[video_lib] clip {} url={:?} in={} out={} pos={} track={} dur={}",
            info.id, info.url, info.in_frame, info.out_frame, info.position, info.track_id, info.duration
        );
        infos.push(info);
    }

    // Collapse linked video+audio pairs (same source, same trim, same position).
    // Kdenlive selects both when you click a clip; we only want one logical entry.
    infos.sort_by_key(|i| (i.position, i.track_id));
    let mut deduped: Vec<TimelineClipInfo> = Vec::new();
    for info in infos {
        let is_dup = deduped.iter().any(|d| {
            d.url == info.url
                && d.position == info.position
                && d.in_frame == info.in_frame
                && d.out_frame == info.out_frame
        });
        if !is_dup {
            deduped.push(info);
        }
    }
    let infos = deduped;
    eprintln!("[video_lib] after dedupe: {} logical clip(s)", infos.len());

    let track_id = infos[0].track_id;
    if let Some(bad) = infos.iter().find(|i| i.track_id != track_id) {
        return Err(AppError::new(
            format!(
                "clips span multiple tracks (clip {} is on track {}, others on {})",
                bad.id, bad.track_id, track_id
            ),
            "kdenlive::pull",
            file!(),
        ));
    }

    for w in infos.windows(2) {
        let prev_end = w[0].position + w[0].duration;
        if prev_end != w[1].position {
            return Err(AppError::new(
                format!(
                    "clips {} and {} aren't glued (prev ends at {}, next starts at {})",
                    w[0].id, w[1].id, prev_end, w[1].position
                ),
                "kdenlive::pull",
                file!(),
            ));
        }
    }

    std::fs::create_dir_all(output_dir)
        .map_err(|e| AppError::new(format!("mkdir {output_dir:?}: {e}"), "kdenlive::pull", file!()))?;

    let start_position = infos[0].position;
    let base_id = infos[0].id;

    let temp_file = if infos.len() == 1 {
        let info = &infos[0];
        let stem = sanitize_stem(&info.name);
        let start_sec = info.in_frame as f64 / fps;
        let dur_sec = info.duration as f64 / fps;
        let out = output_dir.join(format!("{stem}_{:.1}s_{:.2}s.mov", start_sec, dur_sec));
        extract_range(&info.url, start_sec, dur_sec, fps, &out)?;
        out
    } else {
        let first = &infos[0];
        let stem = sanitize_stem(&first.name);
        let mut parts = Vec::new();
        for (i, info) in infos.iter().enumerate() {
            let part = output_dir.join(format!(".part_{}_{}.mov", base_id, i));
            extract_range(&info.url, info.in_frame as f64 / fps, info.duration as f64 / fps, fps, &part)?;
            parts.push(part);
        }
        let out = output_dir.join(format!("{stem}_bundle.mov"));
        concat_files(&parts, &out)?;
        for p in &parts {
            let _ = std::fs::remove_file(p);
        }
        out
    };

    eprintln!("[video_lib] pulled {} clip(s) to {:?}", infos.len(), temp_file);
    Ok(PulledClip { original_clips: infos, start_position, track_id, temp_file })
}

fn pulled_cache_dir() -> AppResult<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| AppError::new("HOME not set", "kdenlive::pull::cache_dir", file!()))?;
    let dir = home.join(".cache/video_library/pulled");
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::new(format!("mkdir {dir:?}: {e}"), "kdenlive::pull::cache_dir", file!()))?;
    Ok(dir)
}

fn extract_range(source: &Path, start_sec: f64, duration_sec: f64, fps: f64, out: &Path) -> AppResult<()> {
    let src = source
        .to_str()
        .ok_or_else(|| AppError::new("source path not utf-8", "kdenlive::pull::extract", file!()))?;
    let dst = out
        .to_str()
        .ok_or_else(|| AppError::new("out path not utf-8", "kdenlive::pull::extract", file!()))?;

    eprintln!("[video_lib] extracting {:.3}s @ {:.3}s (fps {}) to {:?}", duration_sec, start_sec, fps, out);

    let status = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-nostats"])
        .args(["-ss", &format!("{:.6}", start_sec), "-i", src, "-t", &format!("{:.6}", duration_sec)])
        .args(["-r", &format!("{fps}")])
        .args(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "16", "-pix_fmt", "yuv420p", "-an"])
        .arg(dst)
        .status()
        .map_err(|e| AppError::new(format!("spawn ffmpeg: {e}"), "kdenlive::pull::extract", file!()))?;

    if !status.success() {
        return Err(AppError::new(format!("ffmpeg exited {status}"), "kdenlive::pull::extract", file!()));
    }
    Ok(())
}

fn concat_files(parts: &[PathBuf], out: &Path) -> AppResult<()> {
    let list_path = out.with_extension("txt");
    let mut list = String::new();
    for p in parts {
        let s = p
            .to_str()
            .ok_or_else(|| AppError::new("part path not utf-8", "kdenlive::pull::concat", file!()))?;
        let escaped = s.replace('\'', "'\\''");
        list.push_str(&format!("file '{}'\n", escaped));
    }
    std::fs::write(&list_path, list)
        .map_err(|e| AppError::new(format!("write concat list: {e}"), "kdenlive::pull::concat", file!()))?;

    let dst = out
        .to_str()
        .ok_or_else(|| AppError::new("out path not utf-8", "kdenlive::pull::concat", file!()))?;
    let list_str = list_path
        .to_str()
        .ok_or_else(|| AppError::new("list path not utf-8", "kdenlive::pull::concat", file!()))?;

    let status = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-nostats", "-f", "concat", "-safe", "0", "-i", list_str, "-c", "copy", dst])
        .status()
        .map_err(|e| AppError::new(format!("spawn concat: {e}"), "kdenlive::pull::concat", file!()))?;

    let _ = std::fs::remove_file(&list_path);

    if !status.success() {
        return Err(AppError::new(
            format!("concat ffmpeg exited {status} (likely resolution mismatch between source clips)"),
            "kdenlive::pull::concat",
            file!(),
        ));
    }
    Ok(())
}

fn sanitize_stem(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let cleaned: String = stem
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    if trimmed.is_empty() { "clip".to_string() } else { trimmed }
}
