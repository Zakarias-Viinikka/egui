use std::path::PathBuf;
use std::time::Duration;

pub struct TimelineState {
    pub duration: Duration,
    pub position: Duration,
    pub video_path: Option<PathBuf>,
}

impl TimelineState {
    pub fn new() -> Self {
        Self { duration: Duration::ZERO, position: Duration::ZERO, video_path: None }
    }

    pub fn on_load(&mut self, duration: Duration, video_path: PathBuf) {
        self.duration = duration;
        self.position = Duration::ZERO;
        self.video_path = Some(video_path);
    }
}
