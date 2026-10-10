use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use crate::surface::SurfaceFile;
use crate::text_diff::{pair_lines, process, DiffRowProcessedForUi};
use crate::trust::SurfaceDiff;

/// Which kind of permission window is on screen. The UI picks the banner
/// title, colours and hint from this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerKind {
    CargoRun,
    Execute,
    Delete,
    Rename,
}

pub struct SurfaceReviewFile {
    pub path: PathBuf,
    pub rows: Vec<DiffRowProcessedForUi>,
}

pub struct SurfaceReview {
    pub root: PathBuf,
    pub files: Vec<SurfaceReviewFile>,
    /// The surface as collected just now. On confirm this becomes the new
    /// trusted record.
    pub new_surface: Vec<SurfaceFile>,
    pub kind: BannerKind,
    pub caption: String,
    responder: Mutex<Option<Sender<bool>>>,
}

impl SurfaceReview {
    pub fn new(
        root: PathBuf,
        diffs: Vec<SurfaceDiff>,
        new_surface: Vec<SurfaceFile>,
        responder: Sender<bool>,
        kind: BannerKind,
        caption: String,
    ) -> Self {
        let files = diffs
            .into_iter()
            .map(|d| {
                let old = d.old_text.unwrap_or_default();
                let rows = process(&pair_lines(&old, &d.new_text));
                SurfaceReviewFile { path: d.path, rows }
            })
            .collect();
        Self {
            root,
            files,
            new_surface,
            kind,
            caption,
            responder: Mutex::new(Some(responder)),
        }
    }

    pub fn respond(&self, accept: bool) {
        if let Ok(mut g) = self.responder.lock() {
            if let Some(tx) = g.take() {
                let _ = tx.send(accept);
            }
        }
    }
}