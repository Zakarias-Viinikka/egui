use eframe::egui;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

struct Entry {
    kind: String,
    text: String,
    count: usize,
}

static LOG: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
static VISIBLE: AtomicBool = AtomicBool::new(true);

const MAX_STORED: usize = 800;
const COUNTER_WINDOW: usize = 200;
const VISUAL_TOTAL: usize = 200;
const VISUAL_PER_KIND: usize = 5;
const COPY_PER_KIND: usize = 50;

pub fn log(kind: impl Into<String>, line: impl Into<String>) {
    let kind = kind.into();
    let text = line.into();
    let mut l = LOG.lock().unwrap();
    if let Some(last) = l.last_mut() {
        if last.kind == kind && last.text == text {
            last.count += 1;
            return;
        }
    }
    l.push(Entry { kind, text, count: 1 });
    if l.len() > MAX_STORED {
        let drop = l.len() - MAX_STORED;
        l.drain(0..drop);
    }
}

pub fn debug_log_ui(ui: &mut egui::Ui) {
    if ui.input(|i| i.key_pressed(egui::Key::L) && i.modifiers.ctrl) {
        VISIBLE.fetch_xor(true, Ordering::Relaxed);
    }
    if !VISIBLE.load(Ordering::Relaxed) {
        return;
    }

    let entries: Vec<(String, String, usize)> = LOG
        .lock()
        .unwrap()
        .iter()
        .map(|e| (e.kind.clone(), e.text.clone(), e.count))
        .collect();

    // counter: per kind, over the latest COUNTER_WINDOW entries
    let start = entries.len().saturating_sub(COUNTER_WINDOW);
    let mut counters: Vec<(String, usize)> = Vec::new();
    for (kind, _, count) in &entries[start..] {
        if let Some(slot) = counters.iter_mut().find(|(k, _)| k == kind) {
            slot.1 += count;
        } else {
            counters.push((kind.clone(), *count));
        }
    }

    // visual: latest first, cap per kind and total
    let mut shown: Vec<(String, String, usize)> = Vec::new();
    let mut per_kind: HashMap<String, usize> = HashMap::new();
    for (kind, text, count) in entries.iter().rev() {
        if shown.len() >= VISUAL_TOTAL {
            break;
        }
        let c = per_kind.entry(kind.clone()).or_insert(0);
        if *c >= VISUAL_PER_KIND {
            continue;
        }
        *c += 1;
        shown.push((kind.clone(), text.clone(), *count));
    }
    shown.reverse();

    // copy: latest first, cap per kind
    let mut copy_lines: Vec<(String, String, usize)> = Vec::new();
    let mut copied: HashMap<String, usize> = HashMap::new();
    for (kind, text, count) in entries.iter().rev() {
        let c = copied.entry(kind.clone()).or_insert(0);
        if *c >= COPY_PER_KIND {
            continue;
        }
        *c += 1;
        copy_lines.push((kind.clone(), text.clone(), *count));
    }
    copy_lines.reverse();
    let mut copy_out = String::new();
    for (kind, text, count) in &copy_lines {
        for _ in 0..*count {
            copy_out.push_str(&format!("[{}] {}\n", kind, text));
        }
    }

    let screen = ui
        .ctx()
        .input(|i| i.raw.screen_rect)
        .unwrap_or(ui.available_rect_before_wrap());
    let width = 520.0;
    let height = 420.0;

    egui::Area::new(egui::Id::new("debug_log_area"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(screen.left() + 8.0, screen.top() + 8.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200))
                .corner_radius(6.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(width, height));
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized([110.0, 32.0], egui::Button::new("Copy full"))
                                .clicked()
                            {
                                ui.ctx().copy_text(copy_out.clone());
                            }
                            if ui
                                .add_sized([80.0, 32.0], egui::Button::new("Clear"))
                                .clicked()
                            {
                                LOG.lock().unwrap().clear();
                            }
                            let total: usize = entries.iter().map(|(_, _, c)| *c).sum();
                            ui.label(
                                egui::RichText::new(format!("{} lines · ctrl+L", total))
                                    .color(egui::Color32::from_gray(180)),
                            );
                        });

                        ui.horizontal_wrapped(|ui| {
                            for (kind, count) in &counters {
                                ui.label(
                                    egui::RichText::new(format!("{}: {}", kind, count))
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(170, 215, 255)),
                                );
                            }
                        });

                        ui.separator();

                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for (kind, text, count) in &shown {
                                    let line = if *count > 1 {
                                        format!("[{}] {} (x{})", kind, text, count)
                                    } else {
                                        format!("[{}] {}", kind, text)
                                    };
                                    ui.label(
                                        egui::RichText::new(line)
                                            .monospace()
                                            .size(11.0)
                                            .color(egui::Color32::from_gray(220)),
                                    );
                                }
                            });
                    });
                });
        });

    ui.ctx().request_repaint_after(Duration::from_millis(200));
}
