use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use app_core::reading_ram::tmp_tester::TmpStats;
use app_core::reading_ram::RamInfo;

use crate::hotkeys::Cmd;

pub const BACKGROUND_OPACITY: f32 = 0.8;

pub struct GlobalData {
    pub background_opacity: f32,
    pub ram_info: Vec<RamInfo>,
    pub selected_id: Option<u64>,
    pub selection_expires_at: Option<std::time::Instant>,
    pub fade_id: Option<u64>,
    pub fade_progress: f32,
    pub confirm_target: Option<(u64, String)>,
    pub hotkey_rx: Receiver<Cmd>,
    pub visible: bool,
    pub tmp_stats: Arc<Mutex<TmpStats>>,
    pub frozen_pids: Vec<u32>,
    pub cmd_tx: std::sync::mpsc::Sender<Cmd>,
    pub ram_monitor_stop: Arc<std::sync::atomic::AtomicBool>,
}
