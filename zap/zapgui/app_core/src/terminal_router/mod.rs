pub mod listener;
pub mod pending_overwrite;
pub mod route_requests;

use crate::chain::Chain;
use std::sync::{Arc, Mutex, OnceLock};

pub const SOCKET_PATH: &str = "/run/user/1000/zap.sock";

/// The pending chain. The UI thread reads it at the top of every frame.
/// A newer chain replaces whatever's here, unreviewed or not.
static PENDING_CHAIN: OnceLock<Mutex<Option<Arc<Chain>>>> = OnceLock::new();

fn cell() -> &'static Mutex<Option<Arc<Chain>>> {
    PENDING_CHAIN.get_or_init(|| Mutex::new(None))
}

/// The eframe context, stored once at startup. egui only runs the app's
/// `ui` function when something asks it to. A hidden window gets no input,
/// so without an explicit wake-up, the UI thread never checks PENDING_CHAIN
/// and the app never learns a chain arrived.
static CTX: OnceLock<eframe::egui::Context> = OnceLock::new();

pub fn set_ctx(ctx: eframe::egui::Context) {
    let _ = CTX.set(ctx);
}

/// Store a chain and wake the UI. Returns the displaced chain if there was
/// one, so the caller can report it.
pub fn stash_pending_chain(chain: Chain) -> Option<Arc<Chain>> {
    let displaced = cell().lock().unwrap().replace(Arc::new(chain));
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
    displaced
}

/// Return a clone of the current chain, if any. Removes nothing.
pub fn current_pending() -> Option<Arc<Chain>> {
    cell().lock().unwrap().clone()
}

/// Empty the slot, but only if it still holds the given chain.
static MENU_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Ask the UI thread to show its menu. Wake it if it's hidden.
pub fn request_menu() {
    MENU_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
}

/// Consume any pending menu request.
pub fn take_menu_request() -> bool {
    MENU_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst)
}

pub fn clear_pending(chain: &Arc<Chain>) {
    let mut guard = cell().lock().unwrap();
    let same = match guard.as_ref() {
        Some(current) => Arc::ptr_eq(current, chain),
        None => false,
    };
    if same {
        *guard = None;
    }
}
