use eframe::egui::Color32;

// screen backdrop
pub const BACKDROP: [f32; 4] = [0.0, 0.0, 0.0, 0.8];
pub const BACKDROP_OPAQUE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 200);

// surfaces
pub const SURFACE: Color32 = Color32::from_rgba_premultiplied(215, 215, 215, 235);
pub const SURFACE_OPAQUE: Color32 = Color32::from_rgba_premultiplied(240, 240, 240, 245);

// text
pub const TEXT: Color32 = Color32::from_gray(30);
pub const TEXT_WEAK: Color32 = Color32::from_gray(120);

// circle menu textboxes
pub const TEXTBOX_BG: Color32 = Color32::from_gray(230);
pub const TEXTBOX_BG_HIGHLIGHT: Color32 = Color32::from_rgb(175, 205, 245);
pub const TEXTBOX_TEXT: Color32 = Color32::from_gray(30);

// input fields
pub const FIELD_BG: Color32 = Color32::from_gray(248);
pub const FIELD_BORDER: Color32 = Color32::from_gray(180);
pub const FIELD_BORDER_HOVER: Color32 = Color32::from_gray(120);
pub const FIELD_TEXT: Color32 = Color32::from_gray(30);
pub const FIELD_HINT: Color32 = Color32::from_gray(140);

// buttons
pub const BUTTON_BG: Color32 = Color32::from_gray(220);
pub const BUTTON_HOVER_BG: Color32 = Color32::from_gray(200);
pub const BUTTON_TEXT: Color32 = Color32::from_gray(30);

// icon buttons (close, back)
pub const ICON_BG_HOVER: Color32 = Color32::from_gray(225);
pub const ICON_FG: Color32 = Color32::from_gray(70);

// circle
pub const CIRCLE_STROKE: Color32 = Color32::from_rgba_premultiplied(30, 30, 30, 128);
pub const CIRCLE_STROKE_GRAY: u8 = 220;
pub const CIRCLE_STROKE_BASE_ALPHA: u8 = 70;
pub const CIRCLE_STROKE_FLOOR_ALPHA: u8 = 18;
pub const CIRCLE_STROKE_DECAY: f32 = 0.45;

// pulse
pub const PULSE: Color32 = Color32::WHITE;
