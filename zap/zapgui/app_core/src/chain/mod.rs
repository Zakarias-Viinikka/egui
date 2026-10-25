//! Chain parsing, play-forward, writing, reporting, whitelist matching and
//! the popup wrapper for zapgui.
//!
//! A chain is a block of create/replace/overwrite commands that zapgui
//! plays forward in memory before any file is touched. Nothing in this
//! module is wired into the rest of app_core yet.

pub mod astgrep;
pub mod parser;
pub mod play;
pub mod popup;
pub mod report;
pub mod step;
pub mod whitelist;
pub mod writer;

pub use parser::{parse, ParseError, ParseFailure};
pub use play::{
    play_forward, AppliedStep, Chain, ChainOutcome, Override, Snapshot, SupersededChain,
};
pub use report::StepStatus;
pub use step::{Step, Verb};
