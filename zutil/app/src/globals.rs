use eframe::egui;
use std::sync::Mutex;

#[derive(Clone)]
pub enum ItemKind {
    PushTemplatesCategories,
    PushTemplatesInCategory(i64),
    OpenTemplate(i64),
    PushPlaceholder(Vec<String>),
    None,
}

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub kind: ItemKind,
}

static LAST_CLICK: Mutex<Option<egui::Pos2>> = Mutex::new(None);
static MENU_STACK: Mutex<Vec<Vec<MenuItem>>> = Mutex::new(Vec::new());

pub fn record_click(pos: egui::Pos2) {
    *LAST_CLICK.lock().unwrap() = Some(pos);
}

pub fn last_click() -> Option<egui::Pos2> {
    *LAST_CLICK.lock().unwrap()
}

pub fn init_menu(items: Vec<MenuItem>) {
    let mut s = MENU_STACK.lock().unwrap();
    s.clear();
    s.push(items);
}

pub fn push_menu(items: Vec<MenuItem>) {
    MENU_STACK.lock().unwrap().push(items);
}

pub fn pop_menu() -> bool {
    let mut s = MENU_STACK.lock().unwrap();
    if s.len() > 1 {
        s.pop();
        true
    } else {
        false
    }
}

pub fn depth() -> usize {
    MENU_STACK.lock().unwrap().len()
}

pub fn current_items() -> Vec<MenuItem> {
    MENU_STACK.lock().unwrap().last().cloned().unwrap_or_default()
}

pub fn current_labels() -> Vec<String> {
    current_items().into_iter().map(|i| i.label).collect()
}

use std::sync::atomic::{AtomicBool, Ordering};

static MODAL_OPEN: AtomicBool = AtomicBool::new(false);

pub fn set_modal_open(open: bool) {
    MODAL_OPEN.store(open, Ordering::Relaxed);
}

pub fn modal_open() -> bool {
    MODAL_OPEN.load(Ordering::Relaxed)
}

static DISABLE_RIGHTCLICK: AtomicBool = AtomicBool::new(false);

pub fn set_disable_rightclick(v: bool) {
    DISABLE_RIGHTCLICK.store(v, Ordering::Relaxed);
}

pub fn disable_rightclick() -> bool {
    DISABLE_RIGHTCLICK.load(Ordering::Relaxed)
}
