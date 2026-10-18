use super::kdenlive_state::KdenliveState;
use super::log::{LogEntry, LogKind};
use super::render_job::{RenderJob, RenderOutcome};
use super::timeline::TimelineState;
use super::video_view::VideoViewState;
use crate::error::{AppError, AppResult};
use crate::thumbnails::{cache, lru};
use crate::video::decoder::VideoDecoder;
use image::{imageops::FilterType, RgbaImage};
use std::path::PathBuf;
use std::time::Duration;
use video_library_core_core::black_and_white::core::BwPreset;

const MAX_LOG: usize = 30;

pub struct VideoLibraryState {
    pub video_view: VideoViewState,
    pub timeline: TimelineState,
    pub render_job: Option<RenderJob>,
    pub picked_path: Option<PathBuf>,
    pub last_error: Option<String>,
    pub bw_preset: BwPreset,
    pub kdenlive: KdenliveState,
    pub log: Vec<LogEntry>,
    pub last_render_output: Option<PathBuf>,
    last_render_seen: Option<RenderOutcome>,
}

impl VideoLibraryState {
    pub fn new() -> Self {
        Self {
            video_view: VideoViewState::new(),
            timeline: TimelineState::new(),
            render_job: None,
            picked_path: None,
            last_error: None,
            bw_preset: BwPreset::Balanced,
            kdenlive: KdenliveState::new(),
            log: Vec::new(),
            last_render_output: None,
            last_render_seen: None,
        }
    }

    pub fn is_rendering(&self) -> bool {
        self.render_job
            .as_ref()
            .is_some_and(|j| matches!(j.outcome(), RenderOutcome::Running))
    }

    pub fn push_log(&mut self, kind: LogKind, text: impl Into<String>) {
        let text: String = text.into();
        if matches!(kind, LogKind::Error) {
            crate::popup::spawn_popup(&text);
        }
        self.log.push(LogEntry::new(kind, text));
        if self.log.len() > MAX_LOG {
            self.log.remove(0);
        }
    }

    pub fn load_video(&mut self, path: PathBuf) -> AppResult<()> {
        let decoder = VideoDecoder::open(&path)?;
        let duration = decoder.duration();
        if duration.is_zero() {
            return Err(AppError::new("video has zero duration", "video_library_state::load_video", file!()));
        }
        self.video_view.on_load(path.clone(), duration, decoder);
        self.timeline.on_load(duration, path.clone());
        self.picked_path = Some(path.clone());
        self.last_error = None;
        self.push_log(LogKind::Success, format!("Loaded {}", path.display()));

        if let Err(e) = self.video_view.scrub_to(Duration::ZERO) {
            self.last_error = Some(format!("first frame decode failed: {e}"));
            self.push_log(LogKind::Error, format!("First frame decode failed: {e}"));
        }

        let thumb_path = path.clone();
        std::thread::spawn(move || spawn_thumbnail_gen(thumb_path, duration));
        Ok(())
    }

    pub fn start_bw_render(&mut self, output: PathBuf) -> AppResult<()> {
        let Some(source) = self.picked_path.clone() else {
            return Err(AppError::new("no video picked", "video_library_state::start_bw_render", file!()));
        };
        let total_us = self.timeline.duration.as_micros().min(u64::MAX as u128) as u64;
        let job = RenderJob::start(source, output.clone(), self.bw_preset, total_us)?;
        self.last_render_output = Some(output.clone());
        self.last_render_seen = None;
        self.push_log(LogKind::Info, format!("Rendering B&W to {}", output.display()));
        self.render_job = Some(job);
        Ok(())
    }

    pub fn render_outcome(&self) -> Option<RenderOutcome> {
        self.render_job.as_ref().map(|j| j.outcome())
    }

    pub fn start_pull_from_kdenlive(&mut self) {
        eprintln!("[video_lib] pull requested");
        if self.kdenlive.is_busy() {
            eprintln!("[video_lib] pull skipped: busy");
            self.push_log(LogKind::Info, "Busy — another job is running");
            return;
        }
        self.push_log(LogKind::Info, "Pulling clip from Kdenlive...");
        self.kdenlive.start_pull();
    }

    pub fn start_push_to_kdenlive(&mut self) {
        if self.kdenlive.is_busy() {
            self.push_log(LogKind::Info, "Busy — another job is running");
            return;
        }
        let Some(pulled) = self.kdenlive.pulled.as_ref() else {
            self.push_log(LogKind::Error, "Send back: no clip was pulled");
            return;
        };
        let source = self
            .last_render_output
            .clone()
            .or_else(|| self.picked_path.clone());
        let Some(source) = source else {
            self.push_log(LogKind::Error, "Send back: no file to send");
            return;
        };
        let (id, track_id, position) = {
            let info = &pulled.info;
            (info.id, info.track_id, info.position)
        };
        self.push_log(
            LogKind::Info,
            format!("Sending {} to Kdenlive (track {track_id}, position {position})...", source.display()),
        );
        self.kdenlive.start_push(source, id, track_id, position);
    }

    /// Called once per frame before drawing. Returns true when visible state changed.
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        let status_before = self.kdenlive.status.clone();

        if let Some(msg) = self.kdenlive.maybe_refresh_status() {
            self.push_log(LogKind::Error, msg);
            changed = true;
        }
        if self.kdenlive.status != status_before {
            changed = true;
        }

        if let Some(pull_result) = self.kdenlive.poll_pull() {
            changed = true;
            match &pull_result {
                Ok(p) => eprintln!("[video_lib] pull result: Ok({:?})", p.temp_file),
                Err(e) => eprintln!("[video_lib] pull result: Err({e})"),
            }
            match pull_result {
                Ok(pulled) => {
                    let temp = pulled.temp_file.clone();
                    let info_name = pulled.info.name.clone();
                    self.kdenlive.pulled = Some(pulled);
                    match self.load_video(temp) {
                        Ok(()) => self.push_log(LogKind::Success, format!("Pulled '{info_name}' from Kdenlive")),
                        Err(e) => self.push_log(LogKind::Error, format!("Pulled but load failed: {e}")),
                    }
                }
                Err(e) => self.push_log(LogKind::Error, format!("Pull failed: {e}")),
            }
        }

        if let Some(push_result) = self.kdenlive.poll_push() {
            changed = true;
            match push_result {
                Ok(new_id) => {
                    self.push_log(LogKind::Success, format!("Sent back to Kdenlive (new clip id {new_id})"));
                    self.kdenlive.pulled = None;
                }
                Err(e) => self.push_log(LogKind::Error, format!("Send back failed: {e}")),
            }
        }

        if let Some(job) = &self.render_job {
            let now = job.outcome();
            if !same_outcome(&self.last_render_seen, &now) {
                match &now {
                    RenderOutcome::Done => self.push_log(LogKind::Success, "Render done"),
                    RenderOutcome::Cancelled => self.push_log(LogKind::Info, "Render cancelled"),
                    RenderOutcome::Failed(msg) => self.push_log(LogKind::Error, format!("Render failed: {msg}")),
                    RenderOutcome::Running => {}
                }
                self.last_render_seen = Some(now);
                changed = true;
            }
        } else {
            self.last_render_seen = None;
        }

        changed
    }
}

fn same_outcome(a: &Option<RenderOutcome>, b: &RenderOutcome) -> bool {
    matches!(
        (a, b),
        (Some(RenderOutcome::Done), RenderOutcome::Done)
            | (Some(RenderOutcome::Cancelled), RenderOutcome::Cancelled)
            | (Some(RenderOutcome::Running), RenderOutcome::Running)
    )
}

fn spawn_thumbnail_gen(video: PathBuf, duration: Duration) {
    if let Err(e) = cache::ensure_cache_dir() {
        eprintln!("thumb cache dir: {e}");
        return;
    }
    let mut decoder = match VideoDecoder::open(&video) {
        Ok(d) => d,
        Err(e) => { eprintln!("thumb decoder open: {e}"); return; }
    };
    for i in 0..cache::THUMB_COUNT {
        let path = cache::thumb_path(&video, i);
        if path.exists() { continue; }
        let ts = if cache::THUMB_COUNT > 1 {
            duration.mul_f64(i as f64 / cache::THUMB_COUNT as f64)
        } else { Duration::ZERO };
        match decoder.frame_at(ts) {
            Ok(frame) => {
                let Some(img) = RgbaImage::from_raw(frame.width, frame.height, frame.rgba) else { continue };
                let (tw, th) = thumb_dims(frame.width, frame.height);
                let resized = image::imageops::resize(&img, tw, th, FilterType::Triangle);
                if let Err(e) = resized.save(&path) { eprintln!("save thumb {i}: {e}"); }
            }
            Err(e) => eprintln!("thumb frame_at {i}: {e}"),
        }
    }
    lru::evict_default();
}

fn thumb_dims(w: u32, h: u32) -> (u32, u32) {
    if w == 0 || h == 0 { return (cache::THUMB_WIDTH, cache::THUMB_HEIGHT); }
    let aspect = w as f32 / h as f32;
    let target = cache::THUMB_WIDTH as f32 / cache::THUMB_HEIGHT as f32;
    if aspect > target {
        let th = (cache::THUMB_WIDTH as f32 / aspect) as u32;
        (cache::THUMB_WIDTH, th.max(1))
    } else {
        let tw = (cache::THUMB_HEIGHT as f32 * aspect) as u32;
        (tw.max(1), cache::THUMB_HEIGHT)
    }
}