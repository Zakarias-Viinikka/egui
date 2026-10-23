use std::process::Command;

fn find_self_dec() -> Option<String> {
    for _ in 0..50 {
        let out = Command::new("xdotool")
            .args(["search", "--name", "^anti_freeze$"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(line) = text.lines().next() {
            let id = line.trim().to_string();
            if !id.is_empty() {
                return Some(id);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    None
}

fn hex_of(dec: &str) -> Option<String> {
    let n: u64 = dec.parse().ok()?;
    Some(format!("0x{:08x}", n))
}

pub fn set_skip_taskbar() {
    // intentionally empty
}

pub fn raise_self() {
    std::thread::spawn(|| {
        let Some(dec) = find_self_dec() else { return };
        let Some(hex) = hex_of(&dec) else { return };
        let _ = Command::new("wmctrl").args(["-ia", &hex]).output();
    });
}