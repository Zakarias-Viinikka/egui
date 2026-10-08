pub mod listener;
pub mod pending_overwrite;
pub mod route_requests;

use pending_overwrite::PendingOverwrite;
use std::sync::{Arc, Mutex, OnceLock};

pub const SOCKET_PATH: &str = "/run/user/1000/zap.sock";

/// Set by the listener thread when an overwrite request arrives. The UI
/// thread reads it at the top of every frame. If two overwrites arrive before
/// the UI ticks, the second replaces the first.
static PENDING_OVERWRITE: OnceLock<Mutex<Option<Arc<PendingOverwrite>>>> = OnceLock::new();

fn cell() -> &'static Mutex<Option<Arc<PendingOverwrite>>> {
    PENDING_OVERWRITE.get_or_init(|| Mutex::new(None))
}

/// The eframe context, stored once at startup. egui only runs the app's `ui`
/// function when something asks it to. A hidden window gets no input, so
/// without an explicit wake-up, the UI thread never checks PENDING_OVERWRITE
/// and the app never learns a diff arrived.
static CTX: OnceLock<eframe::egui::Context> = OnceLock::new();

pub fn set_ctx(ctx: eframe::egui::Context) {
    let _ = CTX.set(ctx);
}

/// Store a pending edit and wake the UI. Returns the path of a displaced
/// pending edit, if there was one and it was for a different file.
pub fn stash_pending(edit: PendingOverwrite) -> Option<String> {
    let new_path = edit.path.clone();
    let displaced = cell().lock().unwrap().replace(Arc::new(edit));
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
    displaced.and_then(|arc| {
        if arc.path == new_path {
            None
        } else {
            Some(arc.path.clone())
        }
    })
}

/// Return a clone of the current pending edit, if any. Removes nothing.
pub fn current_pending() -> Option<Arc<PendingOverwrite>> {
    cell().lock().unwrap().clone()
}

/// Empty the slot, but only if it still holds the given edit.
pub fn clear_pending(edit: &Arc<PendingOverwrite>) {
    let mut guard = cell().lock().unwrap();
    let same = match guard.as_ref() {
        Some(current) => Arc::ptr_eq(current, edit),
        None => false,
    };
    if same {
        *guard = None;
    }
}
