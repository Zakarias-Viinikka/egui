use std::process::Command;

pub fn show(msg: &str, x: i32, y: i32) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("err_msg: current_exe failed: {e} | method: popup::show | file: core/src/popup/mod.rs"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| String::from("err_msg: current_exe has no parent | method: popup::show | file: core/src/popup/mod.rs"))?
        .to_path_buf();
    let bin = dir.join("fading_popup");
    if !bin.exists() {
        return Err(format!("err_msg: binary not found at {} | method: popup::show | file: core/src/popup/mod.rs", bin.display()));
    }
    Command::new(&bin)
        .arg(msg)
        .arg(x.to_string())
        .arg(y.to_string())
        .spawn()
        .map_err(|e| format!("err_msg: spawn failed: {e} | method: popup::show | file: core/src/popup/mod.rs"))?;
    Ok(())
}
