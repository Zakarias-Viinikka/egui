#[derive(Default)]
pub struct Actions {
    pub open_video_picker: bool,
    pub open_bw_save_picker: bool,
    pub cancel_render: bool,
    pub dismiss_render: bool,
    pub pull_from_kdenlive: bool,
    pub push_to_kdenlive: bool,
}