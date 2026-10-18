use crate::error::{AppError, AppResult};
use crate::video::decoder::VideoDecoder;
use std::path::PathBuf;
use std::time::Duration;

pub struct VideoViewState {
    pub loaded_path: Option<PathBuf>,
    pub decoder: Option<VideoDecoder>,
    pub duration: Duration,
    pub current_frame_rgba: Option<(u32, u32, Vec<u8>)>,
    pub frame_id: u64,
}

impl VideoViewState {
    pub fn new() -> Self {
        Self { loaded_path: None, decoder: None, duration: Duration::ZERO, current_frame_rgba: None, frame_id: 0 }
    }

    pub fn on_load(&mut self, path: PathBuf, duration: Duration, decoder: VideoDecoder) {
        self.loaded_path = Some(path);
        self.duration = duration;
        self.decoder = Some(decoder);
        self.current_frame_rgba = None;
        self.frame_id = 0;
    }

    pub fn scrub_to(&mut self, ts: Duration) -> AppResult<()> {
        let decoder = self.decoder.as_mut()
            .ok_or_else(|| AppError::new("no decoder", "video_view::scrub_to", file!()))?;
        let frame = decoder.frame_at(ts)?;
        self.current_frame_rgba = Some((frame.width, frame.height, frame.rgba));
        self.frame_id = self.frame_id.wrapping_add(1);
        Ok(())
    }
}
