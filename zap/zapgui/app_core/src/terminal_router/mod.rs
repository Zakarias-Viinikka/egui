pub mod cargo_run;
pub mod file_trust;
pub mod listener;
pub mod run_file;
pub mod stream_run;
pub mod pending_overwrite;
pub mod retry;
pub mod route_requests;
pub mod surface_review;

use crate::chain::Chain;
use surface_review::SurfaceReview;
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
    stash_pending_arc(Arc::new(chain))
}

/// Same as stash_pending_chain but takes an Arc directly, so a streaming
/// handler can keep its own handle for withdrawing. Returns the displaced
/// chain, if any.
pub fn stash_pending_arc(chain: Arc<Chain>) -> Option<Arc<Chain>> {
    let displaced = cell().lock().unwrap().replace(chain);
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

static SHOW_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Ask the UI thread to make the window visible and raise it, without
/// opening the menu. Used by the "focus-if-busy" request: an external
/// shortcut asks whether zapgui has a review pending, and if so, this is
/// how it gets shown.
pub fn request_show() {
    SHOW_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
}

/// Consume any pending show request.
pub fn take_show_request() -> bool {
    SHOW_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst)
}

/// True if anything is waiting for the user: a pending chain review or a
/// pending surface review. The menu is not counted here because it's UI
/// state, not router state.
pub fn something_waiting() -> bool {
    current_pending().is_some() || current_surface_review().is_some()
}

/// The pending surface review, if any. A chain review or a menu can coexist
/// with this; the UI shows the surface review first when one is present.
static PENDING_SURFACE: OnceLock<Mutex<Option<Arc<SurfaceReview>>>> = OnceLock::new();

fn surface_cell() -> &'static Mutex<Option<Arc<SurfaceReview>>> {
    PENDING_SURFACE.get_or_init(|| Mutex::new(None))
}

/// Store a surface review and wake the UI. Returns false if one is already
/// waiting. The caller waits on the review's channel for the response.
pub fn stash_surface_review(review: SurfaceReview) -> bool {
    let mut g = surface_cell().lock().unwrap();
    if g.is_some() {
        return false;
    }
    *g = Some(Arc::new(review));
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
    true
}

pub fn current_surface_review() -> Option<Arc<SurfaceReview>> {
    surface_cell().lock().unwrap().clone()
}

pub fn clear_surface_review(r: &Arc<SurfaceReview>) {
    let mut g = surface_cell().lock().unwrap();
    let same = g.as_ref().map(|c| Arc::ptr_eq(c, r)).unwrap_or(false);
    if same {
        *g = None;
    }
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

/// Swap the pending chain, but only when the slot still holds `old`.
/// Returns true on swap, false when a newer chain displaced it. Used by
/// the review window's edit-title pen so a swap never wakes the waiter
/// with "replaced".
pub fn replace_pending_if(old: &Arc<Chain>, new: Arc<Chain>) -> bool {
    let mut g = cell().lock().unwrap();
    let same = g.as_ref().map(|c| Arc::ptr_eq(c, old)).unwrap_or(false);
    if !same {
        return false;
    }
    *g = Some(new);
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
    true
}
