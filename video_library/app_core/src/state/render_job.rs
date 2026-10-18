use crate::error::AppResult;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use video_library_core_core::black_and_white::core::{render_bw, BwPreset};
use video_library_core_core::render::progress::Progress;

#[derive(Clone, Debug)]
pub enum RenderOutcome {
    Running,
    Done,
    Cancelled,
    Failed(String),
}

pub struct RenderJob {
    pub progress: Arc<Progress>,
    pub state: Arc<Mutex<RenderOutcome>>,
    pub source: PathBuf,
    pub output: PathBuf,
}

impl RenderJob {
    pub fn start(source: PathBuf, output: PathBuf, preset: BwPreset, total_us: u64) -> AppResult<Self> {
        let progress = Progress::new(total_us);
        let state = Arc::new(Mutex::new(RenderOutcome::Running));

        let p = progress.clone();
        let s = state.clone();
        let src = source.clone();
        let out = output.clone();

        std::thread::spawn(move || {
            let result = render_bw(&src, &out, preset, p.clone());
            let outcome = match result {
                Ok(()) => if p.is_cancelled() { RenderOutcome::Cancelled } else { RenderOutcome::Done },
                Err(e) => if p.is_cancelled() { RenderOutcome::Cancelled } else { RenderOutcome::Failed(e.to_string()) },
            };
            if let Ok(mut guard) = s.lock() { *guard = outcome; }
        });

        Ok(Self { progress, state, source, output })
    }

    pub fn fraction(&self) -> f32 { self.progress.fraction() }

    pub fn outcome(&self) -> RenderOutcome {
        self.state.lock().map(|g| g.clone())
            .unwrap_or(RenderOutcome::Failed("state lock poisoned".to_string()))
    }
}
