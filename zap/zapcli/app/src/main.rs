use app::egui_main;
use std::process::{Command, Stdio};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-free" || a == "--free") {
        launch_detached();
        return Ok(());
    }
    egui_main::run()
}

/// Re-exec self with no args as a detached GUI process. Setsid drops the
/// controlling terminal, and stdout/stdin/stderr go to /dev/null, so the
/// calling shell gets its prompt back immediately. This is what `zap &`
/// would do at a shell prompt.
fn launch_detached() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = Command::new("setsid")
        .arg(&exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}