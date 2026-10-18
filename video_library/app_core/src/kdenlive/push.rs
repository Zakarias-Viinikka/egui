use crate::error::{AppError, AppResult};
use crate::kdenlive::talker::{build_proxy, connect};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

pub fn push_to_kdenlive(source_file: &Path) -> AppResult<String> {
    eprintln!("[video_lib] push: connecting");
    let (conn, name) = connect()?;
    eprintln!("[video_lib] push: connected to {name}");
    let proxy = build_proxy(&conn, &name)?;

    if !source_file.exists() {
        return Err(AppError::new(
            format!("source file does not exist: {}", source_file.display()),
            "kdenlive::push",
            file!(),
        ));
    }
    let size = std::fs::metadata(source_file).map(|m| m.len()).unwrap_or(0);
    eprintln!("[video_lib] push: source file exists, {size} bytes");

    let path_str = source_file
        .to_str()
        .ok_or_else(|| AppError::new("source path not utf-8", "kdenlive::push", file!()))?
        .to_string();

    if let Some(existing) = find_bin_clip_by_url(&proxy, &path_str)? {
        eprintln!("[video_lib] push: already in bin as {existing}");
        return Ok(existing);
    }

    eprintln!("[video_lib] push: importing {path_str}");
    let imported = proxy
        .script_import_media(vec![path_str.clone()], "-1".to_string())
        .map_err(|e| AppError::new(format!("import: {e}"), "kdenlive::push", file!()))?;
    eprintln!("[video_lib] push: import returned {:?}", imported);

    if let Some(id) = imported.first().cloned() {
        return Ok(id);
    }

    // Kdenlive's scriptImportMedia starts the import but doesn't wait for it,
    // so getClipByUrl returns empty on the first call. The file IS being imported.
    // Poll for it to appear in the bin.
    eprintln!("[video_lib] push: import returned empty, polling bin for the URL");
    match wait_for_bin_url(&proxy, &path_str) {
        Ok(id) => {
            eprintln!("[video_lib] push: found bin id {id} after import");
            Ok(id)
        }
        Err(e) => {
            eprintln!("[video_lib] push: bin id unavailable ({e}); file was still imported");
            // Signal "imported but no id" so caller can show a helpful message
            Err(AppError::new(
                format!("imported (bin id not reported by Kdenlive): {path_str}"),
                "kdenlive::push",
                file!(),
            ))
        }
    }
}

fn wait_for_bin_url(
    proxy: &crate::kdenlive::talker::KdenliveMainWindowProxyBlocking<'_>,
    url: &str,
) -> AppResult<String> {
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    let poll = Duration::from_millis(150);
    loop {
        if start.elapsed() > timeout {
            return Err(AppError::new(
                format!("bin id not found within {timeout:?}"),
                "kdenlive::push::wait",
                file!(),
            ));
        }
        if let Some(id) = find_bin_clip_by_url(proxy, url)? {
            return Ok(id);
        }
        thread::sleep(poll);
    }
}

fn find_bin_clip_by_url(
    proxy: &crate::kdenlive::talker::KdenliveMainWindowProxyBlocking<'_>,
    url: &str,
) -> AppResult<Option<String>> {
    let ids = proxy
        .script_get_all_clip_ids()
        .map_err(|e| AppError::new(format!("get_all_clip_ids: {e}"), "kdenlive::push::find_bin", file!()))?;

    for id in ids {
        let props = match proxy.script_get_clip_properties(&id) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let Some(v) = props.get("url") else { continue };
        let Ok(clip_url) = v.downcast_ref::<String>() else { continue };
        if clip_url == url {
            return Ok(Some(id));
        }
    }
    Ok(None)
}