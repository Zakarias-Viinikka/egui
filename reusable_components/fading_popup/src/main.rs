use fontdue::{Font, FontSettings};
use std::thread;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::wrapper::ConnectionExt as _;

const W: u16 = 260;
const H: u16 = 60;
const CORNER_RADIUS: f32 = 6.0;
const BORDER_WIDTH: f32 = 1.5;
const LINGER_MS: u64 = 800;
const FADE_MS: u64 = 500;
const FADE_STEPS: u64 = 25;
const FONT_SIZE: f32 = 18.0;

const BG: (u8, u8, u8) = (0x14, 0x14, 0x14);
const FG: (u8, u8, u8) = (0xF0, 0xF0, 0xF0);
const BORDER: (u8, u8, u8) = (0xF0, 0xF0, 0xF0);

const FONT_BYTES: &[u8] = include_bytes!("../fonts/Ubuntu-Light.ttf");

fn load_font() -> Option<Font> {
    Font::from_bytes(FONT_BYTES, FontSettings::default()).ok()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let body = args.get(1).cloned().unwrap_or_default();
    let x: i16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(800);
    let y: i16 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let mut argb_visual: Option<u32> = None;
    for d in &screen.allowed_depths {
        if d.depth == 32 {
            for v in &d.visuals {
                if v.class == VisualClass::TRUE_COLOR {
                    argb_visual = Some(v.visual_id);
                    break;
                }
            }
        }
        if argb_visual.is_some() {
            break;
        }
    }

    let visual = match argb_visual {
        Some(v) => v,
        None => screen.root_visual,
    };
    let depth: u8 = if argb_visual.is_some() { 32 } else { screen.root_depth };

    let colormap = if argb_visual.is_some() {
        let cm = conn.generate_id()?;
        conn.create_colormap(ColormapAlloc::NONE, cm, screen.root, visual)?;
        cm
    } else {
        screen.default_colormap
    };

    let win = conn.generate_id()?;

    conn.create_window(
        depth,
        win,
        screen.root,
        x,
        y,
        W,
        H,
        0,
        WindowClass::INPUT_OUTPUT,
        visual,
        &CreateWindowAux::new()
            .background_pixel(0)
            .border_pixel(0)
            .colormap(colormap)
            .override_redirect(1)
            .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),
    )?;

    conn.configure_window(win, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE))?;

    let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let above = conn.intern_atom(false, b"_NET_WM_STATE_ABOVE")?.reply()?.atom;
    let skip_taskbar = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR")?.reply()?.atom;
    let skip_pager = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER")?.reply()?.atom;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        net_wm_state,
        AtomEnum::ATOM,
        &[above, skip_taskbar, skip_pager],
    )?;

    let opacity_atom = conn.intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?.reply()?.atom;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        opacity_atom,
        AtomEnum::CARDINAL,
        &[0xFFFF_FFFFu32],
    )?;

    conn.map_window(win)?;
    conn.flush()?;

    let buf = render_content(W as u32, H as u32, &body);
    let mut drawn = false;
    let start = Instant::now();
    let mut faded = false;

    loop {
        while let Some(event) = conn.poll_for_event()? {
            match event {
                Event::Expose(_) => {
                    if put_buffer(&conn, win, W, H, depth, &buf).is_ok() {
                        let _ = conn.sync();
                        drawn = true;
                    }
                }
                Event::ButtonPress(_) => {
                    let _ = conn.destroy_window(win);
                    let _ = conn.flush();
                    return Ok(());
                }
                _ => {}
            }
        }

        if !drawn {
            if put_buffer(&conn, win, W, H, depth, &buf).is_ok() {
                let _ = conn.sync();
            }
            drawn = true;
        }

        let _ = conn.configure_window(win, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE));
        let _ = conn.flush();

        if !faded && start.elapsed() >= Duration::from_millis(LINGER_MS) {
            faded = true;
            let step_ms = FADE_MS / FADE_STEPS;
            for i in 0..=FADE_STEPS {
                let alpha = 1.0 - (i as f64 / FADE_STEPS as f64);
                let val = (0xFFFF_FFFFu64 as f64 * alpha) as u32;
                let _ = conn.change_property32(
                    PropMode::REPLACE,
                    win,
                    opacity_atom,
                    AtomEnum::CARDINAL,
                    &[val],
                );
                let _ = conn.flush();
                thread::sleep(Duration::from_millis(step_ms));
            }
            break;
        }

        thread::sleep(Duration::from_millis(16));
    }

    let _ = conn.destroy_window(win);
    let _ = conn.flush();
    Ok(())
}

fn render_content(w: u32, h: u32, text: &str) -> Vec<u32> {
    let wf = w as f32;
    let hf = h as f32;
    let r = CORNER_RADIUS;
    let bw = BORDER_WIDTH;

    let mut buf = vec![0u32; (w * h) as usize];

    // paint background + border first
    for py in 0..h {
        for px in 0..w {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;
            let outer = rounded_alpha(fx, fy, wf, hf, r);
            let inner = rounded_alpha(
                fx - bw,
                fy - bw,
                wf - bw * 2.0,
                hf - bw * 2.0,
                (r - bw).max(0.0),
            );
            let border_cov = outer * (1.0 - inner);
            let bg_cov = outer * inner;

            let (rr, gg, bb) = (
                (BG.0 as f32 * bg_cov + BORDER.0 as f32 * border_cov).round() as u32,
                (BG.1 as f32 * bg_cov + BORDER.1 as f32 * border_cov).round() as u32,
                (BG.2 as f32 * bg_cov + BORDER.2 as f32 * border_cov).round() as u32,
            );
            let ai = (outer * 255.0).round() as u32;
            let rp = (rr * ai) / 255;
            let gp = (gg * ai) / 255;
            let bp = (bb * ai) / 255;
            buf[(py * w + px) as usize] = (ai << 24) | (rp << 16) | (gp << 8) | bp;
        }
    }

    // then draw text on top
    if let Some(font) = load_font() {
        if let Some(lm) = font.horizontal_line_metrics(FONT_SIZE) {
            let ascent = lm.ascent;
            let descent = lm.descent;

            let mut pen_x: i32 = 0;
            let mut glyphs: Vec<(fontdue::Metrics, Vec<u8>, i32)> = Vec::new();
            for ch in text.chars() {
                let (m, bmp) = font.rasterize(ch, FONT_SIZE);
                glyphs.push((m, bmp, pen_x + m.xmin));
                pen_x += m.advance_width.round() as i32;
            }
            let text_w = pen_x;
            let start_x = (w as i32 - text_w) / 2;
            let baseline_y = (h as i32 / 2) + ((ascent + descent) / 2.0).round() as i32;

            for (m, bmp, gx) in glyphs {
                let bitmap_top_rel = -(m.ymin + m.height as i32);
                for row in 0..m.height {
                    for col in 0..m.width {
                        let cov = bmp[row * m.width + col];
                        if cov == 0 {
                            continue;
                        }
                        let px = start_x + gx + col as i32;
                        let py = baseline_y + bitmap_top_rel + row as i32;
                        if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                            continue;
                        }
                        let idx = (py as u32 * w + px as u32) as usize;
                        let existing = buf[idx];
                        let ea = (existing >> 24) & 0xFF;
                        if ea == 0 {
                            continue;
                        }
                        let er = (existing >> 16) & 0xFF;
                        let eg = (existing >> 8) & 0xFF;
                        let eb = existing & 0xFF;

                        let a = boost_curve(cov as u32) as f32 / 255.0;
                        let nr = (FG.0 as f32 * a + er as f32 * (1.0 - a)).round() as u32;
                        let ng = (FG.1 as f32 * a + eg as f32 * (1.0 - a)).round() as u32;
                        let nb = (FG.2 as f32 * a + eb as f32 * (1.0 - a)).round() as u32;
                        buf[idx] = (ea << 24) | (nr << 16) | (ng << 8) | nb;
                    }
                }
            }
        }
    }

    buf
}

fn rounded_alpha(px: f32, py: f32, w: f32, h: f32, r: f32) -> f32 {
    if px < 0.0 || py < 0.0 || px > w || py > h {
        return 0.0;
    }
    if px < r && py < r {
        return circle_coverage(r - px, r - py, r);
    }
    if px > w - r && py < r {
        return circle_coverage(px - (w - r), r - py, r);
    }
    if px < r && py > h - r {
        return circle_coverage(r - px, py - (h - r), r);
    }
    if px > w - r && py > h - r {
        return circle_coverage(px - (w - r), py - (h - r), r);
    }
    1.0
}

fn circle_coverage(dx: f32, dy: f32, r: f32) -> f32 {
    let d = (dx * dx + dy * dy).sqrt();
    if d <= r - 0.5 {
        1.0
    } else if d >= r + 0.5 {
        0.0
    } else {
        r + 0.5 - d
    }
}

fn boost_curve(v: u32) -> u32 {
    let t = v as f32 / 255.0;
    let curved = t.powf(0.85);
    (curved * 255.0).round() as u32
}

fn put_buffer<C: Connection>(
    conn: &C,
    drawable: Drawable,
    w: u16,
    h: u16,
    depth: u8,
    buf: &[u32],
) -> Result<(), Box<dyn std::error::Error>> {
    let gc = conn.generate_id()?;
    conn.create_gc(gc, drawable, &CreateGCAux::new())?;

    let mut data: Vec<u8> = Vec::with_capacity(buf.len() * 4);
    for px in buf {
        data.push((*px & 0xFF) as u8);
        data.push(((*px >> 8) & 0xFF) as u8);
        data.push(((*px >> 16) & 0xFF) as u8);
        data.push(((*px >> 24) & 0xFF) as u8);
    }

    conn.put_image(
        ImageFormat::Z_PIXMAP,
        drawable,
        gc,
        w,
        h,
        0,
        0,
        0,
        depth,
        &data,
    )?;
    conn.free_gc(gc)?;
    Ok(())
}
