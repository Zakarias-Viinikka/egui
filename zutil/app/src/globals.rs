use eframe::egui;
use std::sync::Mutex;

#[derive(Clone)]
pub enum ItemKind {
    PushTemplatesCategories,
    PushTemplatesInCategory(i64),
    OpenTemplate(i64),
    PushPromptCategories,
    PushPromptsInCategory(i64),
    CopyPrompt(i64),
    PushTextCategories,
    PushTextsInCategory(i64),
    PushTerminalCategories,
    PushTerminalsInCategory(i64),
    OpenTextEditor(i64),
    ViewText(i64),
    PushProjects,
    OpenProjectTerminal(i64),
    EditProject(i64),
    NewPlus,
    None,
}

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub kind: ItemKind,
    pub counter: u32,
    pub shortcut: Option<String>,
    /// What this item is in the `shortcuts` table, e.g. ("main_nav", "Templates").
    pub owner: (&'static str, String),
}

#[derive(Clone)]
pub enum RebuildKind {
    Root,
    TemplatesCategories,
    TemplatesInCategory(i64),
    Prompts,
    TextCategories,
    TextsInCategory(i64),
    TerminalCategories,
    TerminalsInCategory(i64),
    Projects,
}

static LAST_CLICK: Mutex<Option<egui::Pos2>> = Mutex::new(None);
static MENU_STACK: Mutex<Vec<(RebuildKind, Vec<MenuItem>)>> = Mutex::new(Vec::new());

pub fn record_click(pos: egui::Pos2) {
    *LAST_CLICK.lock().unwrap() = Some(pos);
}

pub fn last_click() -> Option<egui::Pos2> {
    *LAST_CLICK.lock().unwrap()
}

pub fn init_menu(items: Vec<MenuItem>) {
    let mut s = MENU_STACK.lock().unwrap();
    s.clear();
    s.push((RebuildKind::Root, items));
}

pub fn push_menu(kind: RebuildKind, items: Vec<MenuItem>) {
    MENU_STACK.lock().unwrap().push((kind, items));
}

pub fn current_rebuild_kind() -> Option<RebuildKind> {
    MENU_STACK.lock().unwrap().last().map(|(k, _)| k.clone())
}

pub fn set_top_items(items: Vec<MenuItem>) {
    let mut s = MENU_STACK.lock().unwrap();
    if let Some((_, top)) = s.last_mut() {
        *top = items;
    }
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
    MENU_STACK
        .lock()
        .unwrap()
        .last()
        .map(|(_, items)| items.clone())
        .unwrap_or_default()
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

pub fn stack_kinds() -> Vec<RebuildKind> {
    MENU_STACK
        .lock()
        .unwrap()
        .iter()
        .map(|(k, _)| k.clone())
        .collect()
}
