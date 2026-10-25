//! A tiny flag that lets a second zapcli launch raise the running one.
//! The listener in egui_main sets it when a second launch connects and
//! sends "focus"; the UI in drawing_requirements takes it once per frame
//! and issues a viewport Focus.

use std::sync::atomic::{AtomicBool, Ordering};

static FOCUS: AtomicBool = AtomicBool::new(false);

pub fn request_focus() {
    FOCUS.store(true, Ordering::SeqCst);
}

pub fn take_focus() -> bool {
    FOCUS.swap(false, Ordering::SeqCst)
}
