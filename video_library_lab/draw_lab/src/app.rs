use crate::canvas;
use crate::strokes::{StrokeList, Tool};
use app_core::state::video_view::VideoViewState;
use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

pub struct DrawLab {
    pub video: VideoViewState,
    pub path_input: String,
    pub timeline_position: Duration,
    pub tool: Tool,
    pub brush_size: f32,
    pub strokes: StrokeList,
    pub strokes_by_frame: HashMap<usize, StrokeList>,
    pub current_frame_index: usize,
    pub texture: Option<egui::TextureHandle>,
    pub last_frame_id: Option<u64>,
    pub guess_box: Option<(f32, f32, f32, f32)>,
    pub guess_frame: Option<usize>,
    pub mask_texture: Option<egui::TextureHandle>,
    pub mask_frame_id: Option<u64>,
    mask_cache: HashMap<usize, (u32, u32, Vec<u8>)>,
    pub tracking: Option<Child>,
    pub tracking_out: Option<PathBuf>,
    pub tracking_status: Option<String>,
    pub last_error: Option<String>,
    pub playing: bool,
    pub last_tick: Instant,
    pub last_requested_frame: Option<usize>,
}

impl Default for DrawLab {
    fn default() -> Self {
        Self {
            video: VideoViewState::new(),
            path_input: String::new(),
            timeline_position: Duration::ZERO,
            tool: Tool::Brush,
            brush_size: 8.0,
            strokes: StrokeList::new(),
            strokes_by_frame: HashMap::new(),
            current_frame_index: 0,
            texture: None,
            last_frame_id: None,
            guess_box: None,
            guess_frame: None,
            mask_texture: None,
            mask_frame_id: None,
            mask_cache: HashMap::new(),
            tracking: None,
            tracking_out: None,
            tracking_status: None,
            last_error: None,
            playing: false,
            last_tick: Instant::now(),
            last_requested_frame: None,
        }
    }
}

impl DrawLab {
    fn frame_index_at(&self, ts: Duration) -> usize {
        if self.video.fps > 0.0 {
            (ts.as_secs_f64() * self.video.fps).round() as usize
        } else {
            0
        }
    }

    fn set_frame(&mut self, new_idx: usize) {
        if new_idx == self.current_frame_index {
            return;
        }
        let old = self.current_frame_index;
        let strokes = std::mem::take(&mut self.strokes);
        if strokes.is_empty() {
            self.strokes_by_frame.remove(&old);
        } else {
            self.strokes_by_frame.insert(old, strokes);
        }
        self.current_frame_index = new_idx;
        self.strokes = self.strokes_by_frame.remove(&new_idx).unwrap_or_default();
        self.guess_box = None;
        self.mask_texture = None;
        self.mask_frame_id = None;
    }

    fn run_guess(&mut self) {
        self.last_error = None;
        let Some((w, h, rgba)) = self.video.current_frame_rgba.as_ref() else {
            self.last_error = Some("no frame to guess from".into());
            return;
        };
        let (w, h, rgba) = (*w, *h, rgba.clone());
        let Some(video_path) = self.video.loaded_path.clone() else {
            self.last_error = Some("no video path".into());
            return;
        };
        let mut brush = Vec::new();
        let mut eraser = Vec::new();
        for s in self.strokes.iter() {
            match s.tool {
                Tool::Brush => brush.extend_from_slice(&s.points),
                Tool::Eraser => eraser.extend_from_slice(&s.points),
            }
        }
        match crate::guess::run(w, h, &rgba, &brush, &eraser) {
            Ok(mask) => {
                let out = crate::guess::seed_path(&video_path, self.current_frame_index);
                match crate::guess::save_png(&out, w, h, &mask) {
                    Ok(()) => {
                        self.guess_box = self.strokes.brush_bounds();
                        self.guess_frame = Some(self.current_frame_index);
                        eprintln!(
                            "[guess] wrote {} (seed frame {})",
                            out.display(),
                            self.current_frame_index
                        );
                    }
                    Err(e) => self.last_error = Some(e),
                }
            }
            Err(e) => self.last_error = Some(e),
        }
    }

    fn run_track(&mut self) {
        self.last_error = None;
        if self.tracking.is_some() {
            self.last_error = Some("tracking already running".into());
            return;
        }
        let Some(video_path) = self.video.loaded_path.clone() else {
            self.last_error = Some("no video loaded".into());
            return;
        };
        let Some(proxy_path) = self.video.proxy_path().cloned() else {
            self.last_error = Some("no proxy".into());
            return;
        };
        let Some(seed_frame) = self.guess_frame else {
            self.last_error = Some("guess a mask first".into());
            return;
        };
        let mask_path = crate::guess::seed_path(&video_path, seed_frame);
        if !mask_path.exists() {
            self.last_error = Some("guess a mask first".into());
            return;
        }

        let stem = video_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video");
        let parent = video_path.parent().unwrap_or_else(|| Path::new("."));
        let out_dir = parent.join(format!("{stem}_tracking")).join("masks");

        if let Err(e) = std::fs::create_dir_all(&out_dir) {
            self.last_error = Some(format!("mkdir: {e}"));
            return;
        }

        let python = venv_python();
        let script = track_script();
        if !python.exists() {
            self.last_error = Some(format!("python not found: {}", python.display()));
            return;
        }
        if !script.exists() {
            self.last_error = Some(format!("track.py not found: {}", script.display()));
            return;
        }

        let mut cmd = std::process::Command::new(&python);
        cmd.arg(&script)
            .arg("--video")
            .arg(&proxy_path)
            .arg("--mask")
            .arg(&mask_path)
            .arg("--out")
            .arg(&out_dir)
            .arg("--seed-frame")
            .arg(seed_frame.to_string());
        cmd.env("PYTORCH_CUDA_ALLOC_CONF", "expandable_segments:True");
        cmd.stdout(std::process::Stdio::inherit());
        cmd.stderr(std::process::Stdio::inherit());

        match cmd.spawn() {
            Ok(child) => {
                self.tracking = Some(child);
                self.tracking_out = Some(out_dir);
                self.tracking_status = Some(format!("tracking from frame {seed_frame}…"));
                self.mask_cache.clear();
                self.mask_frame_id = None;
                eprintln!("[track] spawned, seed frame {seed_frame}");
            }
            Err(e) => {
                self.last_error = Some(format!("spawn: {e}"));
            }
        }
    }

    fn poll_tracking(&mut self) {
        let Some(child) = self.tracking.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    let count = self
                        .tracking_out
                        .as_ref()
                        .map(|p| count_pngs(p))
                        .unwrap_or(0);
                    self.tracking_status = Some(format!("{count} masks on disk"));
                    self.mask_frame_id = None;
                } else {
                    self.tracking_status = Some(format!("failed: {status}"));
                    self.last_error = Some(format!("track exited with {status}"));
                }
                self.tracking = None;
            }
            Ok(None) => {}
            Err(e) => {
                self.last_error = Some(format!("poll: {e}"));
                self.tracking = None;
            }
        }
    }

    fn load_mask_for_current_frame(&mut self, ctx: &egui::Context) {
        self.mask_texture = None;
        let Some(out_dir) = self.tracking_out.clone() else {
            return;
        };
        let idx = self.current_frame_index;
        let path = out_dir.join(format!("{idx:05}.png"));
        if !path.exists() {
            return;
        }

        let (w, h, rgba) = if let Some(cached) = self.mask_cache.get(&idx) {
            cached.clone()
        } else {
            match load_mask_rgba(&path) {
                Ok(v) => {
                    self.mask_cache.insert(idx, v.clone());
                    v
                }
                Err(e) => {
                    eprintln!("[mask] load failed: {e}");
                    return;
                }
            }
        };

        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        self.mask_texture =
            Some(ctx.load_texture(format!("mask_{idx}"), img, egui::TextureOptions::LINEAR));
    }

    fn load(&mut self) {
        self.last_error = None;
        self.playing = false;
        if self.tracking.is_some() {
            self.last_error = Some("stop tracking before loading a new clip".into());
            return;
        }
        let cleaned = self
            .path_input
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .trim();
        let path = PathBuf::from(cleaned);
        if path.as_os_str().is_empty() {
            self.last_error = Some("empty path".into());
            return;
        }
        let proxy = match app_core::video::proxy::ensure(&path) {
            Ok(p) => p,
            Err(e) => {
                self.last_error = Some(format!("proxy: {e}"));
                return;
            }
        };
        if let Err(e) = self.video.on_load(path.clone(), proxy) {
            self.last_error = Some(format!("open: {e}"));
            return;
        }
        self.timeline_position = Duration::ZERO;
        self.video.request_frame(Duration::ZERO);
        self.texture = None;
        self.last_frame_id = None;
        self.strokes = StrokeList::new();
        self.strokes_by_frame.clear();
        self.current_frame_index = 0;
        self.guess_box = None;
        self.guess_frame = None;
        self.mask_texture = None;
        self.mask_frame_id = None;
        self.mask_cache.clear();
        self.tracking_out = None;
        self.tracking_status = None;
        self.last_requested_frame = None;

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
        if let Some(parent) = path.parent() {
            let new_path = parent.join(format!("{stem}_tracking")).join("masks");
            let old_path = parent.join(format!("{stem}_masks"));
            let existing = if new_path.exists() {
                Some(new_path)
            } else if old_path.exists() {
                Some(old_path)
            } else {
                None
            };
            if let Some(dir) = existing {
                let count = count_pngs(&dir);
                if count > 0 {
                    self.tracking_out = Some(dir);
                    self.tracking_status = Some(format!("{count} masks loaded from disk"));
                }
            }

            let tracking_dir = parent.join(format!("{stem}_tracking"));
            if let Some(latest) = latest_seed(&tracking_dir) {
                self.guess_frame = Some(latest);
                self.tracking_status = Some(match &self.tracking_status {
                    Some(s) => format!("{s}; seed at frame {latest}"),
                    None => format!("seed at frame {latest}"),
                });
            }
        }
    }

    fn request_if_new_frame(&mut self, ts: Duration) {
        let idx = self.frame_index_at(ts);
        if self.last_requested_frame != Some(idx) {
            self.last_requested_frame = Some(idx);
            self.video.request_frame(ts);
        }
    }
}

fn is_video(p: &Path) -> bool {
    matches!(
        p.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("mp4" | "webm" | "mov" | "mkv" | "avi" | "m4v" | "ogv" | "flv" | "wmv")
    )
}

fn venv_python() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join(".venv/bin/python"))
        .unwrap_or_else(|| PathBuf::from("python3"))
}

fn track_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("track.py"))
        .unwrap_or_else(|| PathBuf::from("track.py"))
}

fn latest_seed(dir: &Path) -> Option<usize> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut best: Option<usize> = None;
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name = name.to_str()?;
        if let Some(stripped) = name.strip_prefix("seed_").and_then(|s| s.strip_suffix(".png")) {
            if let Ok(n) = stripped.parse::<usize>() {
                best = Some(best.map_or(n, |b| b.max(n)));
            }
        }
    }
    best
}

fn count_pngs(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("png"))
                .count()
        })
        .unwrap_or(0)
}

fn load_mask_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let img = image::open(path)
        .map_err(|e| format!("open: {e}"))?
        .to_luma8();
    let (w, h) = img.dimensions();
    let mut rgba = vec![0u8; (w as usize) * (h as usize) * 4];
    for (i, px) in img.pixels().enumerate() {
        if px[0] > 127 {
            rgba[i * 4] = 255;
            rgba[i * 4 + 1] = 40;
            rgba[i * 4 + 2] = 40;
            rgba[i * 4 + 3] = 140;
        }
    }
    Ok((w, h, rgba))
}

impl eframe::App for DrawLab {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_tracking();

        let dropped = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .find(|p| p.is_file() && is_video(p))
        });
        if let Some(path) = dropped {
            self.path_input = path.display().to_string();
            self.load();
        }

        if self.video.poll() {
            if let Some(err) = self.video.error.clone() {
                self.last_error = Some(format!("decode: {err}"));
            }
        }

        if self.playing && self.video.fps > 0.0 {
            let frame_dur = Duration::from_secs_f64(1.0 / self.video.fps);
            if self.last_tick.elapsed() >= frame_dur {
                self.last_tick = Instant::now();
                let next_ts = self.timeline_position + frame_dur;
                if next_ts >= self.video.duration {
                    self.playing = false;
                } else {
                    self.timeline_position = next_ts;
                    let idx = self.frame_index_at(next_ts);
                    self.set_frame(idx);
                    self.request_if_new_frame(next_ts);
                }
            }
        }

        if self.video.current_frame_rgba.is_some()
            && self.mask_frame_id != Some(self.video.frame_id)
        {
            self.load_mask_for_current_frame(ui.ctx());
            self.mask_frame_id = Some(self.video.frame_id);
        }

        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.add(egui::TextEdit::singleline(&mut self.path_input).desired_width(400.0));
            if ui.button("Load").clicked() {
                self.load();
            }
            if let Some(s) = &self.tracking_status {
                ui.separator();
                ui.label(s);
            }
        });

        ui.horizontal(|ui| {
            if ui.selectable_label(self.tool == Tool::Brush, "Brush").clicked() {
                self.tool = Tool::Brush;
            }
            if ui.selectable_label(self.tool == Tool::Eraser, "Eraser").clicked() {
                self.tool = Tool::Eraser;
            }
            ui.add(
                egui::Slider::new(&mut self.brush_size, 1.0..=60.0)
                    .text("brush")
                    .show_value(true),
            );
            ui.separator();
            let has_strokes = !self.strokes.is_empty();
            if ui.add_enabled(has_strokes, egui::Button::new("↶ Undo point")).clicked() {
                self.strokes.undo_tail();
                self.guess_box = None;
            }
            if ui.add_enabled(has_strokes, egui::Button::new("↶ Undo stroke")).clicked() {
                self.strokes.undo_stroke();
                self.guess_box = None;
            }
            if ui.add_enabled(has_strokes, egui::Button::new("Clear")).clicked() {
                self.strokes.clear();
                self.guess_box = None;
            }
            ui.separator();
            if ui.add_enabled(has_strokes, egui::Button::new("Guess")).clicked() {
                self.run_guess();
            }
            if ui
                .add_enabled(self.tracking.is_none(), egui::Button::new("Track"))
                .clicked()
            {
                self.run_track();
            }
        });

        if let Some(err) = &self.last_error {
            ui.colored_label(ui.visuals().error_fg_color, err);
        }

        ui.horizontal(|ui| {
            let dur = self.video.duration.as_secs_f64();
            if dur > 0.0 {
                let max = (dur - 0.04).max(0.0);
                let mut pos = self.timeline_position.as_secs_f64().clamp(0.0, max);

                let play_label = if self.playing { "⏸" } else { "▶" };
                if ui.button(play_label).clicked() {
                    self.playing = !self.playing;
                    self.last_tick = Instant::now();
                }

                let reserved = 180.0;
                let width = (ui.available_width() - reserved).max(120.0);
                ui.spacing_mut().slider_width = width;
                let resp = ui.add(
                    egui::Slider::new(&mut pos, 0.0..=max)
                        .show_value(false)
                        .trailing_fill(true),
                );
                if resp.changed() {
                    self.playing = false;
                    self.timeline_position = Duration::from_secs_f64(pos);
                    let new_idx = self.frame_index_at(self.timeline_position);
                    self.set_frame(new_idx);
                    self.request_if_new_frame(self.timeline_position);
                }
                ui.label(format!("frame {}", self.current_frame_index));
            } else {
                ui.label("no video loaded");
            }
        });

        ui.separator();

        let avail = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());
        canvas::draw(ui, self, rect, &response);

        if self.video.has_pending() || self.playing {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        if self.tracking.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }
    }
}