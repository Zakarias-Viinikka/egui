use crate::error::{AppError, AppResult};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

const METHOD: &str = "video::proxy::ensure";
const PROXY_HEIGHT: u32 = 720;

/// Returns the path to a 720p all-keyframe proxy for `source`, transcoding
/// on first call and caching afterwards. Cache key is source path + mtime + size,
/// so edits to the source produce a fresh proxy.
pub fn ensure(source: &Path) -> AppResult<PathBuf> {
    let cache_dir = proxy_cache_dir()?;
    let key = cache_key(source)?;
    let proxy_path = cache_dir.join(format!("{key}.mp4"));
    if proxy_path.exists() {
        return Ok(proxy_path);
    }
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| AppError::new(format!("{e}"), METHOD, file!()))?;
    transcode(source, &proxy_path)?;
    Ok(proxy_path)
}

fn proxy_cache_dir() -> AppResult<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| AppError::new("HOME not set", METHOD, file!()))?;
    Ok(PathBuf::from(home).join(".cache/video_library/proxies"))
}

fn cache_key(source: &Path) -> AppResult<String> {
    let meta = std::fs::metadata(source)
        .map_err(|e| AppError::new(format!("{e}"), METHOD, file!()))?;
    let mtime = meta
        .modified()
        .map_err(|e| AppError::new(format!("{e}"), METHOD, file!()))?
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::new(format!("{e}"), METHOD, file!()))?
        .as_secs();

    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in source.as_os_str().as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    for b in mtime.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    for b in meta.len().to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    Ok(format!("{h:016x}"))
}

fn transcode(source: &Path, dest: &Path) -> AppResult<()> {
    let tmp = dest.with_extension("tmp.mp4");
    let scale = format!("scale=-2:{PROXY_HEIGHT}");
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(source)
        .args([
            "-vf", &scale,
            "-g", "1",
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-crf", "23",
            "-c:a", "aac",
            "-b:a", "128k",
        ])
        .arg(&tmp)
        .status()
        .map_err(|e| AppError::new(format!("{e}"), METHOD, file!()))?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        return Err(AppError::new(
            format!("ffmpeg exited with {status}"),
            METHOD,
            file!(),
        ));
    }
    std::fs::rename(&tmp, dest)
        .map_err(|e| AppError::new(format!("{e}"), METHOD, file!()))?;
    Ok(())
}
