use crate::error::{AppError, AppResult};
use crate::video::decode_worker::DecodeWorker;
use crate::video::decoder::VideoDecoder;
use crate::video::frame::Frame;
use std::path::PathBuf;
use std::time::Duration;

pub struct VideoViewState {
    pub loaded_path: Option<PathBuf>,
    pub duration: Duration,
    pub fps: f64,
    pub current_frame_rgba: Option<(u32, u32, Vec<u8>)>,
    pub frame_id: u64,
    pub error: Option<String>,
    worker: Option<DecodeWorker>,
    request_gen: u64,
    applied_gen: u64,
    thumb_decoder: Option<VideoDecoder>,
    proxy_path: Option<PathBuf>,
}

impl VideoViewState {
    pub fn new() -> Self {
        Self {
            loaded_path: None,
            duration: Duration::ZERO,
            fps: 0.0,
            current_frame_rgba: None,
            frame_id: 0,
            error: None,
            worker: None,
            request_gen: 0,
            applied_gen: 0,
            thumb_decoder: None,
            proxy_path: None,
        }
    }

    pub fn on_load(&mut self, source: PathBuf, proxy: PathBuf) -> AppResult<()> {
        self.worker = None;
        self.thumb_decoder = None;
        self.current_frame_rgba = None;
        self.frame_id = 0;
        self.request_gen = 0;
        self.applied_gen = 0;
        self.error = None;
        self.duration = Duration::ZERO;
        self.fps = 0.0;

        let (worker, duration, fps) = DecodeWorker::spawn(source.clone(), proxy.clone())?;
        self.worker = Some(worker);
        self.duration = duration;
        self.fps = fps;
        self.loaded_path = Some(source);
        self.proxy_path = Some(proxy);
        Ok(())
    }

    pub fn request_frame(&mut self, ts: Duration) {
        self.request_gen = self.request_gen.wrapping_add(1);
        let generation = self.request_gen;
        if let Some(worker) = self.worker.as_ref() {
            worker.request(generation, ts);
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut results = Vec::new();
        if let Some(worker) = self.worker.as_ref() {
            while let Some(r) = worker.try_recv() {
                results.push(r);
            }
        }
        let mut changed = false;
        for (generation, result) in results {
            if generation != self.request_gen {
                continue;
            }
            match result {
                Ok(frame) => {
                    self.current_frame_rgba = Some((frame.width, frame.height, frame.rgba));
                    self.frame_id = self.frame_id.wrapping_add(1);
                    self.applied_gen = generation;
                    self.error = None;
                    changed = true;
                }
                Err(e) => {
                    self.error = Some(format!("{e}"));
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn proxy_path(&self) -> Option<&PathBuf> {
        self.proxy_path.as_ref()
    }

    pub fn has_pending(&self) -> bool {
        self.request_gen > self.applied_gen
    }

    pub fn decode_frame_at(&mut self, ts: Duration) -> AppResult<Frame> {
        let source = self.loaded_path.clone().ok_or_else(|| {
            AppError::new("no video loaded", "video_view::decode_frame_at", file!())
        })?;
        let proxy = self.proxy_path.clone().ok_or_else(|| {
            AppError::new("no proxy", "video_view::decode_frame_at", file!())
        })?;
        if self.thumb_decoder.is_none() {
            self.thumb_decoder = Some(VideoDecoder::open_proxy(&source, &proxy)?);
        }
        let Some(decoder) = self.thumb_decoder.as_mut() else {
            return Err(AppError::new(
                "thumb decoder missing",
                "video_view::decode_frame_at",
                file!(),
            ));
        };
        decoder.frame_at(ts)
    }
}