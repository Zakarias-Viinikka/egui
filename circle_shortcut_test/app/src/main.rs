use eframe::egui;

#[derive(Clone)]
struct Item {
    label: &'static str,
    shortcut: Option<&'static str>,
    action: Action,
}

#[derive(Clone, Copy)]
enum Action {
    Push(usize),
    Do(&'static str),
}

struct Circle {
    name: &'static str,
    items: Vec<Item>,
}

fn tree() -> Vec<Circle> {
    vec![
        Circle { name: "root", items: vec![
            Item { label: "Templates", shortcut: Some("g"), action: Action::Push(1) },
            Item { label: "Text",      shortcut: Some("t"), action: Action::Push(2) },
        ]},
        Circle { name: "templates", items: vec![
            Item { label: "git", shortcut: Some("g"), action: Action::Push(3) },
            Item { label: "adb", shortcut: Some("a"), action: Action::Push(4) },
        ]},
        Circle { name: "text", items: vec![
            Item { label: "notes", shortcut: Some("n"), action: Action::Push(5) },
        ]},
        Circle { name: "templates/git", items: vec![
            Item { label: "view diff", shortcut: Some("g"), action: Action::Do("view diff") },
            Item { label: "commit",    shortcut: Some("c"), action: Action::Do("commit") },
        ]},
        Circle { name: "templates/adb", items: vec![
            Item { label: "devices", shortcut: Some("v"), action: Action::Do("list devices") },
        ]},
        Circle { name: "text/notes", items: vec![
            Item { label: "first",  shortcut: Some("1"), action: Action::Do("view note 1") },
            Item { label: "second", shortcut: Some("2"), action: Action::Do("view note 2") },
        ]},
    ]
}

struct App {
    tree: Vec<Circle>,
    stack: Vec<usize>,
    last_action: String,
}

impl App {
    fn new() -> Self {
        Self { tree: tree(), stack: vec![0], last_action: "-".into() }
    }

    fn current(&self) -> usize {
        *self.stack.last().unwrap()
    }

    fn find_shortcut(&self, circle: usize, combo: &str) -> Option<usize> {
        self.tree[circle].items.iter().position(|i| i.shortcut == Some(combo))
    }

    fn fire(&mut self, circle: usize, idx: usize) {
        let item = &self.tree[circle].items[idx];
        match item.action {
            Action::Push(n) => {
                self.stack.push(n);
                self.last_action = format!("nav -> {}", self.tree[n].name);
            }
            Action::Do(s) => {
                self.last_action = format!("do \"{}\"", s);
            }
        }
    }

    // Walk the shortcut chain from the current circle without navigating.
    // Stop when we hit a Do, or the shortcut isn't in the next circle.
    fn descend(&mut self, combo: &str) {
        let mut circle = self.current();
        loop {
            let Some(idx) = self.find_shortcut(circle, combo) else {
                break;
            };
            match self.tree[circle].items[idx].action {
                Action::Do(_) => {
                    self.fire(circle, idx);
                    break;
                }
                Action::Push(next) => {
                    circle = next;
                }
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // read key events
        let mut pressed: Option<(String, bool)> = None;
        ui.input(|i| {
            for ev in &i.events {
                if let egui::Event::Key { key, pressed: true, modifiers, .. } = ev {
                    if matches!(key, egui::Key::ControlLeft | egui::Key::ControlRight) {
                        continue;
                    }
                    let k = format!("{:?}", key).to_lowercase();
                    pressed = Some((k, modifiers.ctrl));
                    break;
                }
            }
        });

        if let Some((combo, ctrl)) = pressed {
            if ctrl {
                self.descend(&combo);
            } else if let Some(idx) = self.find_shortcut(self.current(), &combo) {
                self.fire(self.current(), idx);
            }
        }

        let screen = ui.available_rect_before_wrap();
        let center = screen.center();
        let r = 180.0;
        let n = self.tree[self.current()].items.len();
        for i in 0..n {
            let a = -std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::TAU / (n as f32);
            let p = egui::pos2(center.x + a.cos() * r, center.y + a.sin() * r);
            let rect = egui::Rect::from_center_size(p, egui::vec2(150.0, 44.0));
            let it = &self.tree[self.current()].items[i];
            let label = match it.shortcut {
                Some(s) => format!("{}  [{}]", it.label, s),
                None => it.label.to_string(),
            };
            if ui.put(rect, egui::Button::new(label)).clicked() {
                let c = self.current();
                self.fire(c, i);
            }
        }

        // back
        if self.stack.len() > 1 {
            let rect = egui::Rect::from_min_size(
                screen.min + egui::vec2(20.0, 20.0),
                egui::vec2(40.0, 40.0),
            );
            if ui.put(rect, egui::Button::new("<")).clicked() {
                self.stack.pop();
            }
        }

        // right-click pops
        if ui.input(|i| i.pointer.secondary_clicked()) {
            if self.stack.len() > 1 {
                self.stack.pop();
            }
        }

        // hud
        ui.painter().text(
            egui::pos2(screen.left() + 20.0, screen.bottom() - 60.0),
            egui::Align2::LEFT_TOP,
            format!(
                "circle: {}  |  depth: {}",
                self.tree[self.current()].name,
                self.stack.len()
            ),
            egui::FontId::proportional(14.0),
            egui::Color32::from_gray(200),
        );
        ui.painter().text(
            egui::pos2(screen.left() + 20.0, screen.bottom() - 40.0),
            egui::Align2::LEFT_TOP,
            format!("last: {}", self.last_action),
            egui::FontId::proportional(14.0),
            egui::Color32::from_gray(220),
        );

        ui.ctx().request_repaint();
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "circle_shortcut_test",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
