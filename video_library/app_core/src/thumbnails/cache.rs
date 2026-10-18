use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub const THUMB_COUNT: usize = 100;
pub const THUMB_WIDTH: u32 = 160;
pub const THUMB_HEIGHT: u32 = 90;
pub const LIMIT_BYTES: u64 = 500 * 1024 * 1024;

pub fn cache_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".cache/video_library/thumbnails")
}

pub fn ensure_cache_dir() -> std::io::Result<()> {
    fs::create_dir_all(cache_dir())
}

fn key_for(video: &Path) -> u64 {
    let meta = fs::metadata(video).ok();
    let mtime_nanos = meta.as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64).unwrap_or(0);
    let size = meta.map(|m| m.len()).unwrap_or(0);

    let mut hasher = DefaultHasher::new();
    video.to_string_lossy().hash(&mut hasher);
    mtime_nanos.hash(&mut hasher);
    size.hash(&mut hasher);
    hasher.finish()
}

pub fn thumb_path(video: &Path, index: usize) -> PathBuf {
    cache_dir().join(format!("{:016x}_{:04}.png", key_for(video), index))
}

pub fn load_thumb_rgba(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some((w, h, rgba.into_raw()))
}

pub fn save_thumb_from_frame(path: &Path, frame_w: u32, frame_h: u32, rgba: &[u8]) -> Option<()> {
    let img = image::RgbaImage::from_raw(frame_w, frame_h, rgba.to_vec())?;
    let (tw, th) = thumb_dims(frame_w, frame_h);
    let resized = image::imageops::resize(&img, tw, th, image::imageops::FilterType::Triangle);
    resized.save(path).ok()?;
    Some(())
}

pub fn thumb_dims(w: u32, h: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (THUMB_WIDTH, THUMB_HEIGHT);
    }
    let aspect = w as f32 / h as f32;
    let target = THUMB_WIDTH as f32 / THUMB_HEIGHT as f32;
    if aspect > target {
        let th = (THUMB_WIDTH as f32 / aspect) as u32;
        (THUMB_WIDTH, th.max(1))
    } else {
        let tw = (THUMB_HEIGHT as f32 * aspect) as u32;
        (tw.max(1), THUMB_HEIGHT)
    }
}
