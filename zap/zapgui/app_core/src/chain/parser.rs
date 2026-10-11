//! The chain parser now lives in zap_wire::chain, shared with zapcli so
//! both sides enforce the same rules. This module is a re-export.

pub use zap_wire::chain::{parse, ParseError};