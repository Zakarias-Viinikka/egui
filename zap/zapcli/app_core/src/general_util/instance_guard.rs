//! A tiny flag that lets a second zapcli launch raise the running one.
//! The listener in egui_main sets it when a second launch connects and
//! sends "focus"; the UI in drawing_requirements takes it once per frame
//! and issues a viewport Focus. The same socket also carries `cd <path>`
//! from the `zap p` shell wrapper, which stores a pending cwd for the UI
//! to pick up on its next frame.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static FOCUS: AtomicBool = AtomicBool::new(false);
static PENDING_CD: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn request_focus() {
    FOCUS.store(true, Ordering::SeqCst);
}

pub fn take_focus() -> bool {
    FOCUS.swap(false, Ordering::SeqCst)
}

/// Set the cwd for the running zapcli and request focus so the change is
/// visible. Called by the listener when the wrapper sends `cd <path>`.
pub fn request_cd(path: PathBuf) {
    if let Ok(mut g) = PENDING_CD.lock() {
        *g = Some(path);
    }
    request_focus();
}

/// Take the pending cwd, if any. Called once per frame by the UI.
pub fn take_cd() -> Option<PathBuf> {
    PENDING_CD.lock().ok().and_then(|mut g| g.take())
}
