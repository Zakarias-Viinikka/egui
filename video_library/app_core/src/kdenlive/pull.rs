use crate::error::{AppError, AppResult};
use crate::kdenlive::info::TimelineClipInfo;
use crate::kdenlive::talker::{build_proxy, connect};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct PulledClip {
    pub info: TimelineClipInfo,
    pub temp_file: PathBuf,
}

pub fn pull_from_kdenlive() -> AppResult<PulledClip> {
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
    if ids.len() > 1 {
        return Err(AppError::new(
            format!("select exactly one clip ({} selected)", ids.len()),
            "kdenlive::pull",
            file!(),
        ));
    }
    let clip_id = ids[0];

    let info_map = proxy
        .script_get_timeline_clip_info(clip_id)
        .map_err(|e| AppError::new(format!("get_clip_info: {e}"), "kdenlive::pull", file!()))?;
    let info = TimelineClipInfo::from_dbus(&info_map)?;
    eprintln!("[video_lib] clip {} url={:?} in={} out={}", info.id, info.url, info.in_frame, info.out_frame);

    let fps = proxy
        .script_get_project_fps()
        .map_err(|e| AppError::new(format!("get_fps: {e}"), "kdenlive::pull", file!()))?;
    if fps <= 0.0 {
        return Err(AppError::new(format!("bad fps {fps}"), "kdenlive::pull", file!()));
    }

    let start_sec = info.in_frame as f64 / fps;
    let duration_frames = info.out_frame - info.in_frame + 1;
    let duration_sec = duration_frames as f64 / fps;

    let cache = pulled_cache_dir()?;
    let out = cache.join(format!("pull_{}.mov", clip_id));
    eprintln!("[video_lib] extracting {:.2}s @ {:.2}s to {:?}", duration_sec, start_sec, out);
    extract_range(&info.url, start_sec, duration_sec, &out)?;
    eprintln!("[video_lib] extracted");

    Ok(PulledClip { info, temp_file: out })
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

fn extract_range(source: &Path, start_sec: f64, duration_sec: f64, out: &Path) -> AppResult<()> {
    let src = source
        .to_str()
        .ok_or_else(|| AppError::new("source path not utf-8", "kdenlive::pull::extract", file!()))?;
    let dst = out
        .to_str()
        .ok_or_else(|| AppError::new("out path not utf-8", "kdenlive::pull::extract", file!()))?;

    let status = Command::new("ffmpeg")
        .args(["-y", "-ss", &format!("{:.6}", start_sec), "-i", src, "-t", &format!("{:.6}", duration_sec)])
        .args(["-c:v", "prores_ks", "-profile:v", "3", "-vendor", "apl0", "-pix_fmt", "yuv422p10le", "-an"])
        .arg(dst)
        .status()
        .map_err(|e| AppError::new(format!("spawn ffmpeg: {e}"), "kdenlive::pull::extract", file!()))?;

    if !status.success() {
        return Err(AppError::new(format!("ffmpeg exited {status}"), "kdenlive::pull::extract", file!()));
    }
    Ok(())
}
