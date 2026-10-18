use eframe::egui;

pub struct HomePageUiState {
    pub video_texture: Option<egui::TextureHandle>,
    pub last_uploaded_frame_id: Option<u64>,
    pub thumbnail_textures: Vec<Option<egui::TextureHandle>>,
}

impl HomePageUiState {
    pub fn new() -> Self {
        Self {
            video_texture: None,
            last_uploaded_frame_id: None,
            thumbnail_textures: vec![None; app_core::thumbnails::cache::THUMB_COUNT],
        }
    }
}
