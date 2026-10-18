use opencv::core::{Mat, Rect, Scalar, CV_8UC1};
use opencv::imgproc::{grab_cut, GC_INIT_WITH_MASK};
use opencv::prelude::*;
use std::path::{Path, PathBuf};

/// Runs GrabCut on a single RGBA frame using brush/eraser strokes as seeds.
/// brush = foreground, eraser = background.
/// Returns a binary mask (0 = background, 255 = object) as a raw buffer.
pub fn run(
    frame_w: u32,
    frame_h: u32,
    rgba: &[u8],
    brush: &[(f32, f32)],
    eraser: &[(f32, f32)],
) -> Result<Vec<u8>, String> {
    let w = frame_w as i32;
    let h = frame_h as i32;

    // OpenCV wants BGR, 3-channel.
    let mut bgr = Mat::new_rows_cols_with_default(h, w, opencv::core::CV_8UC3, Scalar::all(0.0))
        .map_err(|e| format!("alloc bgr: {e}"))?;
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = (y * w as usize + x) * 4;
            let r = rgba[i];
            let g = rgba[i + 1];
            let b = rgba[i + 2];
            *bgr.at_2d_mut::<opencv::core::Vec3b>(y as i32, x as i32)
                .map_err(|e| format!("at: {e}"))? = opencv::core::Vec3b::from([b, g, r]);
        }
    }

    // GrabCut mask: 0 = bg, 1 = fg, 2 = probable bg, 3 = probable fg.
    // Anything not touched by strokes is "probable background" so GrabCut
    // knows it is allowed to change it.
    let mut mask = Mat::new_rows_cols_with_default(h, w, CV_8UC1, Scalar::all(2.0))
        .map_err(|e| format!("alloc mask: {e}"))?;

    let mut set = |pts: &[(f32, f32)], value: f64| -> Result<(), String> {
        for (nx, ny) in pts {
            let x = (nx * w as f32) as i32;
            let y = (ny * h as f32) as i32;
            if x >= 0 && y >= 0 && x < w && y < h {
                *mask
                    .at_2d_mut::<u8>(y, x)
                    .map_err(|e| format!("mask at: {e}"))? = value as u8;
            }
        }
        Ok(())
    };
    set(eraser, 0.0)?;
    set(brush, 1.0)?;

    if brush.is_empty() {
        return Err("no brush strokes to guess from".into());
    }

    let mut bgd = Mat::default();
    let mut fgd = Mat::default();
    let rect = Rect::new(0, 0, w, h);

    grab_cut(
        &mut bgr,
        &mut mask,
        rect,
        &mut bgd,
        &mut fgd,
        5,
        GC_INIT_WITH_MASK,
    )
    .map_err(|e| format!("grabcut: {e}"))?;

    // Convert GrabCut mask (1 = fg) to 0/255 binary.
    let mut out = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let v = *mask.at_2d::<u8>(y, x).map_err(|e| format!("read mask: {e}"))?;
            if v == 1 {
                out[(y * w + x) as usize] = 255;
            }
        }
    }
    Ok(out)
}

pub fn save_png(path: &Path, w: u32, h: u32, data: &[u8]) -> Result<(), String> {
    let img = image::GrayImage::from_raw(w, h, data.to_vec())
        .ok_or_else(|| "mask size mismatch".to_string())?;
    img.save(path).map_err(|e| format!("save png: {e}"))
}

pub fn seed_path(video: &Path, frame: usize) -> PathBuf {
    let stem = video.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
    let dir = video
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}_tracking"));
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("seed_{frame:05}.png"))
}
