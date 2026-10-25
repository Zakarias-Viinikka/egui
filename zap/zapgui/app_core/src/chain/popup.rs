//! The popup that tells the reviewer a decision was recorded. It is a
//! separate binary, `fading_popup`, that owns its own X window, so
//! zapgui's side is only "spawn it with these numbers". Every call site
//! that also copies to the clipboard goes through `notify_copied`, so
//! the " · copied" / "copy failed" suffix cannot drift.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// Width of the popup in points, computed from the text and clamped.
const POPUP_MIN_W: f32 = 220.0;
const POPUP_MAX_W: f32 = 560.0;
const POPUP_PAD: f32 = 32.0;
/// Approximate width per character in the popup's 18 px Ubuntu Light.
/// Close enough for centring; the popup clips its own text past the
/// right edge.
const POPUP_CHAR_W: f32 = 10.5;
/// Popup height in points. 40 tall so the 18 px text breathes; sits
/// flush with the review's top margin so a review can stay on screen
/// underneath.
const POPUP_H: f32 = 40.0;
/// Top offset so the popup's bottom just meets the review's content.
const POPUP_TOP_OFFSET: f32 = 0.0;

/// The last spawned popup, so a burst replaces rather than stacks.
static LAST: Mutex<Option<Child>> = Mutex::new(None);

#[cfg(test)]
thread_local! {
    static RECORDED: std::cell::RefCell<Vec<String>> =
        std::cell::RefCell::new(Vec::new());
    static RECORD: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// The last recorded popup text. Tests only.
#[cfg(test)]
pub fn last_text() -> Option<String> {
    RECORDED.with(|r| r.borrow().last().cloned())
}

/// Drain the recorded texts. Tests only.
#[cfg(test)]
pub fn take_texts() -> Vec<String> {
    RECORDED.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

/// Turn recording off so a test can actually spawn the binary (the
/// popup_gallery test does).
#[cfg(test)]
pub fn disable_recording() {
    RECORD.with(|c| c.set(false));
}

pub fn show(text: &str) {
    #[cfg(test)]
    {
        if RECORD.with(|c| c.get()) {
            RECORDED.with(|r| r.borrow_mut().push(text.to_string()));
            return;
        }
    }
    show_with_size(text, None);
}

/// Write the clipboard first, then show a popup whose headline says
/// whether the copy worked. Every call site uses this, so the suffix is
/// always the same.
pub fn notify_copied(headline: &str, clipboard_text: &str) {
    let ok = crate::clipboard::copy_to_clipboard(clipboard_text).is_ok();
    let line = if ok {
        format!("{headline} \u{00b7} copied")
    } else {
        format!("{headline} \u{00b7} copy failed")
    };
    show(&line);
}

pub fn show_with_size(text: &str, screen_size: Option<(f32, f32)>) {
    let Some(binary) = find_binary() else { return };
    let (mx, my, mw, mh) = monitor_rect();
    let (w, _h) = screen_size.unwrap_or((mw, mh));
    let popup_w = width_for(text);
    let x = (mx + w / 2.0 - popup_w / 2.0).round() as i32;
    let y = (my + work_area_top() + POPUP_TOP_OFFSET).round() as i32;

    if let Ok(mut g) = LAST.lock() {
        if let Some(mut prev) = g.take() {
            let _ = prev.kill();
            let _ = prev.wait();
        }
    }

    let child = Command::new(&binary)
        .arg(text)
        .arg(x.to_string())
        .arg(y.to_string())
        .arg(popup_w.round().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Ok(c) = child {
        if let Ok(mut g) = LAST.lock() {
            *g = Some(c);
        }
    }
}

fn width_for(text: &str) -> f32 {
    let n = text.chars().count() as f32;
    (n * POPUP_CHAR_W + POPUP_PAD).clamp(POPUP_MIN_W, POPUP_MAX_W)
}

/// Top-left and width/height of the monitor in use: the one with `*` in
/// `xrandr --listmonitors`, else the first one, else the combined size
/// from `xdotool`, else 1920x1080. No caching: zapgui is a long-lived
/// daemon and monitors change.
fn monitor_rect() -> (f32, f32, f32, f32) {
    if let Ok(out) = Command::new("xrandr").args(["--listmonitors"]).output() {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut first: Option<(f32, f32, f32, f32)> = None;
            for line in text.lines().skip(1) {
                // " 0: +*eDP-1 2560/340x1440/190+0+0  eDP-1"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 3 {
                    continue;
                }
                let marker = parts[1];
                let geom = parts[2];
                if let Some(r) = parse_monitor_geom(geom) {
                    if marker.contains('*') {
                        return r;
                    }
                    if first.is_none() {
                        first = Some(r);
                    }
                }
            }
            if let Some(r) = first {
                return r;
            }
        }
    }
    if let Ok(o) = Command::new("xdotool").args(["getdisplaygeometry"]).output() {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut p = text.split_whitespace();
            if let (Some(w), Some(h)) = (p.next(), p.next()) {
                if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                    return (0.0, 0.0, w, h);
                }
            }
        }
    }
    (0.0, 0.0, 1920.0, 1080.0)
}

/// Format: "<w>/<wmm>x<h>/<hmm>+<x>+<y>".
fn parse_monitor_geom(s: &str) -> Option<(f32, f32, f32, f32)> {
    let (wh, rest) = s.split_once('+')?;
    let mut offsets = rest.split('+');
    let xoff: f32 = offsets.next()?.parse().ok()?;
    let yoff: f32 = offsets.next()?.parse().ok()?;
    let (wpart, hpart) = wh.split_once('x')?;
    let w: f32 = wpart.split('/').next()?.parse().ok()?;
    let h: f32 = hpart.split('/').next()?.parse().ok()?;
    Some((xoff, yoff, w, h))
}

/// The top of the work area from `_NET_WORKAREA`, so a desktop panel
/// doesn't hide the popup. 0 on any failure.
fn work_area_top() -> f32 {
    let Ok(out) = Command::new("xprop").args(["-root", "_NET_WORKAREA"]).output() else {
        return 0.0;
    };
    if !out.status.success() {
        return 0.0;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let Some((_, after)) = text.split_once('=') else {
        return 0.0;
    };
    let nums: Vec<&str> = after.split(',').map(|s| s.trim()).collect();
    nums.get(1)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0)
}

fn find_binary() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let local = PathBuf::from(&home).join(".local/bin/fading_popup");
    if local.exists() {
        return Some(local);
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if dir.is_empty() {
                continue;
            }
            let candidate = PathBuf::from(dir).join("fading_popup");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_recording(on: bool) {
        RECORD.with(|c| c.set(on));
    }

    #[test]
    fn notify_copied_says_copied_when_the_write_worked() {
        let _ = take_texts();
        set_recording(true);
        notify_copied("Applied 3 steps", "some clipboard payload");
        let texts = take_texts();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0], "Applied 3 steps \u{00b7} copied");
    }

    #[test]
    fn width_grows_with_text_and_is_clamped() {
        assert_eq!(width_for("x"), POPUP_MIN_W);
        let long = "x".repeat(200);
        assert_eq!(width_for(&long), POPUP_MAX_W);
        assert!(width_for("Apply now") > POPUP_MIN_W);
    }

    #[test]
    fn monitor_geom_parses_the_xrandr_line() {
        let r = parse_monitor_geom("2560/340x1440/190+0+0").unwrap();
        assert_eq!(r, (0.0, 0.0, 2560.0, 1440.0));
        let r = parse_monitor_geom("1920/521x1080/293+2560+0").unwrap();
        assert_eq!(r, (2560.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    #[ignore]
    fn popup_gallery() {
        // Fires every distinct popup text 700 ms apart, then a burst of
        // three within 400 ms (must replace, not stack), then the
        // longest text. Run with:
        //   cargo test -p app_core popup_gallery -- --ignored --nocapture
        set_recording(false);
        let texts = [
            "Applied 1 step \u{00b7} copied",
            "Applied 3 steps \u{00b7} copied",
            "Denied all \u{00b7} copied",
            "Edit failed \u{00b7} copied",
            "Retry failed \u{00b7} copied",
            "Copied line \u{00b7} copied",
            "Copied side \u{00b7} copied",
            "Copied diff \u{00b7} copied",
            "Replaced \u{00b7} copied",
            "Change was made \u{00b7} copied",
            "Applied 3 steps \u{00b7} copy failed",
            "Edit failed \u{00b7} copy failed",
        ];
        for t in texts {
            show(t);
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
        for t in ["one", "two", "three"] {
            show(t);
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
        show("edit /a/very/long/path/that/goes/on/forever/and/ever/src/lib.rs \u{00b7} copied");
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}