pub mod listener;
pub mod route_requests;

use crate::text_diff::post_process::DiffRowProcessedForUi;
use std::sync::{Mutex, OnceLock};

pub const SOCKET_PATH: &str = "/run/user/1000/zap.sock";

/// Set by the listener thread when an overwrite request produces a new diff.
/// The UI thread picks it up at the top of its frame and replaces what it's
/// showing. If two overwrites arrive before the UI ticks, the second replaces
/// the first — that's fine, the user only cares about the latest.
static PENDING_DIFF: OnceLock<Mutex<Option<Vec<DiffRowProcessedForUi>>>> = OnceLock::new();

fn cell() -> &'static Mutex<Option<Vec<DiffRowProcessedForUi>>> {
    PENDING_DIFF.get_or_init(|| Mutex::new(None))
}

/// The eframe context, stored once at startup. egui only runs the app's `ui`
/// function when something asks it to. A hidden window gets no input, so
/// without an explicit wake-up, the UI thread never checks PENDING_DIFF and
/// the app never learns a diff arrived.
static CTX: OnceLock<eframe::egui::Context> = OnceLock::new();

pub fn set_ctx(ctx: eframe::egui::Context) {
    let _ = CTX.set(ctx);
}

pub fn stash_diff(rows: Vec<DiffRowProcessedForUi>) {
    *cell().lock().unwrap() = Some(rows);
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
}

pub fn take_diff() -> Option<Vec<DiffRowProcessedForUi>> {
    cell().lock().unwrap().take()
}
