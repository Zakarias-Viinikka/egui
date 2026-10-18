use std::time::Duration;

/// A decoded frame as flat RGBA8 pixels.
/// egui-agnostic — the caller uploads `rgba` into a texture.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub timestamp: Duration,
}