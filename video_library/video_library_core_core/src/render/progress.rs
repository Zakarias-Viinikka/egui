use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

pub struct Progress {
    completed_us: AtomicU64,
    total_us: AtomicU64,
    cancelled: AtomicBool,
}

impl Progress {
    pub fn new(total_us: u64) -> Arc<Self> {
        Arc::new(Self {
            completed_us: AtomicU64::new(0),
            total_us: AtomicU64::new(total_us),
            cancelled: AtomicBool::new(false),
        })
    }

    pub fn fraction(&self) -> f32 {
        let completed = self.completed_us.load(Ordering::Relaxed);
        let total = self.total_us.load(Ordering::Relaxed);
        if total == 0 { 0.0 } else { (completed as f32 / total as f32).clamp(0.0, 1.0) }
    }

    pub fn set_completed_us(&self, us: u64) { self.completed_us.store(us, Ordering::Relaxed); }
    pub fn request_cancel(&self) { self.cancelled.store(true, Ordering::Relaxed); }
    pub fn is_cancelled(&self) -> bool { self.cancelled.load(Ordering::Relaxed) }
}
