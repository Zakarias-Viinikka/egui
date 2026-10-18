use crate::error::{AppError, AppResult};
use crate::render::progress::Progress;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

const METHOD_RENDER: &str = "black_and_white::core::render_bw";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BwPreset {
    Balanced,
    Faster,
    WayFaster,
    Quality,
}

impl BwPreset {
    pub fn ffmpeg_preset(self) -> &'static str {
        match self {
            Self::Balanced => "medium",
            Self::Faster => "veryfast",
            Self::WayFaster => "ultrafast",
            Self::Quality => "slow",
        }
    }

    pub fn crf(self) -> &'static str {
        match self {
            Self::Balanced => "23",
            Self::Faster => "23",
            Self::WayFaster => "28",
            Self::Quality => "20",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced (default)",
            Self::Faster => "Faster",
            Self::WayFaster => "Way faster, slight quality loss",
            Self::Quality => "Better quality, slower",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Balanced, Self::Faster, Self::WayFaster, Self::Quality]
    }
}

pub fn render_bw(input: &Path, output: &Path, preset: BwPreset, progress: Arc<Progress>) -> AppResult<()> {
    let input_str = input.to_str().ok_or_else(|| AppError::new("input path is not utf-8", METHOD_RENDER, file!()))?;
    let output_str = output.to_str().ok_or_else(|| AppError::new("output path is not utf-8", METHOD_RENDER, file!()))?;

    let mut child = Command::new("ffmpeg")
        .args(["-y", "-i", input_str])
        .args(["-vf", "hue=s=0"])
        .args(["-c:v", "libx264", "-preset", preset.ffmpeg_preset(), "-crf", preset.crf()])
        .args(["-c:a", "aac", "-b:a", "192k"])
        .args(["-progress", "pipe:1", "-nostats", "-loglevel", "error"])
        .arg(output_str)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::new(format!("spawn ffmpeg: {e}"), METHOD_RENDER, file!()))?;

    let stdout = child.stdout.take().ok_or_else(|| AppError::new("no stdout", METHOD_RENDER, file!()))?;
    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                let _ = child.kill(); let _ = child.wait();
                return Err(AppError::new(format!("read progress: {e}"), METHOD_RENDER, file!()));
            }
        };
        if progress.is_cancelled() {
            let _ = child.kill(); let _ = child.wait();
            return Err(AppError::new("cancelled", METHOD_RENDER, file!()));
        }
        if let Some(rest) = line.strip_prefix("out_time_us=") {
            if let Ok(us) = rest.trim().parse::<u64>() { progress.set_completed_us(us); }
        }
    }

    let status = child.wait().map_err(|e| AppError::new(format!("wait: {e}"), METHOD_RENDER, file!()))?;
    if !status.success() {
        if progress.is_cancelled() { return Err(AppError::new("cancelled", METHOD_RENDER, file!())); }
        return Err(AppError::new(format!("ffmpeg exited {status}"), METHOD_RENDER, file!()));
    }
    Ok(())
}
