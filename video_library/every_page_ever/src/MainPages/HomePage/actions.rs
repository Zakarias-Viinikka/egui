#[derive(Default)]
pub struct Actions {
    pub open_video_picker: bool,
    pub start_bw_render: bool,
    pub cancel_render: bool,
    pub dismiss_render: bool,
    pub pull_from_kdenlive: bool,
    pub push_to_kdenlive: bool,
}