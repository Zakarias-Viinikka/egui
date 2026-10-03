use super::super::{Line, Outcome};

pub fn run() -> Outcome {
    let Ok(exe) = std::env::current_exe() else {
        return Outcome::Print(vec![Line::error("settings: cannot find own executable")]);
    };
    let Some(dir) = exe.parent() else {
        return Outcome::Print(vec![Line::error("settings: cannot find own directory")]);
    };
    let settings = dir.join("zap-settings");
    match std::process::Command::new(&settings).spawn() {
        Ok(_) => Outcome::Silent,
        Err(e) => Outcome::Print(vec![Line::error(format!(
            "settings: {}: {e}",
            settings.display()
        ))]),
    }
}
