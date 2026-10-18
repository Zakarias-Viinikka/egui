use super::kdenlive_state::KdenliveState;
use super::log::{LogEntry, LogKind};
use super::render_job::{RenderJob, RenderOutcome};
use super::timeline::TimelineState;
use super::video_view::VideoViewState;
use crate::error::{AppError, AppResult};
use crate::video::decoder::VideoDecoder;
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
    pub last_error_message: Option<String>,
    pub pending_pull_name: Option<String>,
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
            last_error_message: None,
            pending_pull_name: None,
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
            eprintln!("[video_lib] ERROR: {text}");
            crate::popup::spawn_popup(&text);
            self.last_error_message = Some(text.clone());
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

        Ok(())
    }

    pub fn start_bw_render_auto(&mut self) -> AppResult<()> {
        let Some(source) = self.picked_path.clone() else {
            return Err(AppError::new("no video picked", "video_library_state::start_bw_render_auto", file!()));
        };
        let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let parent = source
            .parent()
            .ok_or_else(|| AppError::new("source has no parent dir", "video_library_state::start_bw_render_auto", file!()))?;
        let output = parent.join(format!("{stem}_bw.mp4"));
        let total_us = self.timeline.duration.as_micros().min(u64::MAX as u128) as u64;
        let job = RenderJob::start(source, output.clone(), self.bw_preset, total_us)?;
        self.last_render_output = Some(output.clone());
        self.last_render_seen = None;
        self.push_log(LogKind::Info, format!("Rendering B&W → {}", output.display()));
        self.render_job = Some(job);
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
        self.push_log(LogKind::Info, format!("Rendering B&W → {}", output.display()));
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
        let output_dir = resolve_output_dir();
        self.push_log(
            LogKind::Info,
            format!("Pulling from Kdenlive → {}", output_dir.display()),
        );
        self.kdenlive.start_pull(output_dir);
    }

    pub fn start_push_to_kdenlive(&mut self) {
        eprintln!("[video_lib] push requested");
        if self.kdenlive.is_busy() {
            eprintln!("[video_lib] push skipped: busy");
            self.push_log(LogKind::Info, "Busy — another job is running");
            return;
        }
        let source = self
            .last_render_output
            .clone()
            .or_else(|| self.picked_path.clone());
        let Some(source) = source else {
            self.push_log(LogKind::Error, "Send back: no file to send");
            return;
        };
        self.push_log(
            LogKind::Info,
            format!("Adding {} to Kdenlive project...", source.display()),
        );
        self.kdenlive.start_push(source);
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
                    let count = pulled.original_clips.len();
                    let name = pulled.display_name();
                    self.pending_pull_name = Some(name.clone());
                    self.kdenlive.pulled = Some(pulled);
                    let success_msg = if count == 1 {
                        format!("Pulled '{name}' from Kdenlive")
                    } else {
                        format!("Pulled {count} glued clips as '{name}' from Kdenlive")
                    };
                    match self.load_video(temp) {
                        Ok(()) => self.push_log(LogKind::Success, success_msg),
                        Err(e) => self.push_log(LogKind::Error, format!("Pulled but load failed: {e}")),
                    }
                }
                Err(e) => self.push_log(LogKind::Error, format!("Pull failed: {e}")),
            }
        }

        if let Some(push_result) = self.kdenlive.poll_push() {
            changed = true;
            match push_result {
                Ok(bin_id) => {
                    let msg = format!("Added to Kdenlive bin as {bin_id}");
                    crate::popup::spawn_popup(&msg);
                    self.push_log(
                        LogKind::Success,
                        format!("{msg}. Drag it onto the timeline to replace."),
                    );
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("imported (bin id not reported") {
                        crate::popup::spawn_popup("Added to Kdenlive bin");
                        self.push_log(
                            LogKind::Info,
                            "File imported to Kdenlive bin (bin id not reported; check the bin and drag onto timeline)",
                        );
                    } else {
                        self.push_log(LogKind::Error, format!("Send back failed: {msg}"));
                    }
                }
            }
        }

        let mut clear_render_job = false;
        if let Some(job) = &self.render_job {
            let now = job.outcome();
            if !same_outcome(&self.last_render_seen, &now) {
                match &now {
                    RenderOutcome::Done => self.push_log(LogKind::Success, "Render done"),
                    RenderOutcome::Cancelled => self.push_log(LogKind::Info, "Render cancelled"),
                    RenderOutcome::Failed(msg) => self.push_log(LogKind::Error, format!("Render failed: {msg}")),
                    RenderOutcome::Running => {}
                }
                self.last_render_seen = Some(now.clone());
                changed = true;

                if matches!(now, RenderOutcome::Done | RenderOutcome::Cancelled) {
                    clear_render_job = true;
                }
            }
        } else {
            self.last_render_seen = None;
        }
        if clear_render_job {
            self.render_job = None;
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


fn resolve_output_dir() -> PathBuf {
    if let Ok(project_path) = crate::kdenlive::talker::get_project_path() {
        if !project_path.is_empty() {
            let p = std::path::Path::new(&project_path);
            if let Some(parent) = p.parent() {
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project");
                return parent.join(format!("{stem}_extracts"));
            }
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".cache/video_library/pulled")
}
